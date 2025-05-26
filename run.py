import json
import threading
import time
import subprocess
from pathlib import Path


IP_LIST_PATH = "ip_list.txt"
THREE_NODEJSON_PATH = "3-node.json"
FIVE_NODEJSON_PATH = "5-node.json"
SSH_KEY_PATH = "/path/to/your/key/on/controller/machine.pem"
REMOTE_USER = "ubuntu" 
REMOTE_CONFIG_PATHS = {"holipaxos" : "holipaxos/config/config.json",
                "multipaxos" : "multipaxos/config/config.json",
                "raft" : "raft-kv-example/config/config.json",
                "omnipaxos" : "omnipaxos-kv-store/config.json"}

SERVER_CMDS = {
    "holipaxos": "cd holipaxos && ./run.sh ",
    "multipaxos": "cd multipaxos && ./run.sh ",
    "raft": "cd raft-kv-example && ./run.sh ",
    "omnipaxos": "cd omnipaxos-kv-store && ./run.sh ",
    "copilot-master": "cd copilot/src && ./run_master.sh",
    "copilot-server": "cd copilot/src && ./run_server.sh ",
}

YCSB_CLIENT_CMD = "cd ycsb && ./script/gc/run.sh multipaxos 32 1000000 "

VERSIONS = ["holipaxos", "multipaxos", "copilot", "omnipaxos", "raft"]

IPTABLES_DROP_COMMAND = "sudo iptables -I INPUT -j DROP -s "
IPTABLES_DELETE_COMMAND = "sudo iptables -D INPUT 1"
IPTABLES_ACCEPT_COMMAND = "sudo iptables -I INPUT 1 -j ACCEPT"

client_machine_ip = ""
server_3node = []
server_5node = []
ports_3node = [10000, 11000, 12000]
ports_5node = [10000, 11000, 12000, 13000, 14000]

def read_ips(ip_file):
    with open(ip_file) as f:
        return [line.strip() for line in f if line.strip()]

def update_json_addresses(version, json_path, new_addresses, ports, is_log_experiment=False):
    with open(version + "-config/" + json_path, "r") as f:
        config = json.load(f)

    if version == "raft":
        addresses = [f"http://{ip}:{port}" for ip, port in zip(new_addresses, ports)]
    else:
        addresses = [f"{ip}:{port}" for ip, port in zip(new_addresses, ports)]
    if is_log_experiment and version == "raft":
        config["snapshot_interval"] = 1000000
    config["peers"] = addresses

    with open(json_path, "w") as f:
        json.dump(config, f, indent=2)

def broadcast_file(file_path, ips, remote_user, remote_path, ssh_key):
    for ip in ips:
        target = f"{remote_user}@{ip}:{remote_path}"
        print(f"[+] Sending config to {ip}...")
        try:
            result = subprocess.run([
                "scp",
                "-i", ssh_key,
                file_path,
                target
            ], timeout=10, capture_output=True)
            if result.returncode != 0:
                print(f"[!] Failed to copy to {ip}")
        except Exception as e:
            print(f"[!] Failed to copy to {ip}")

def setup_config(versions, num_servers, is_log_experiment=False):
    for version in versions:
        if version == "copilot":
            continue
        remote_path = REMOTE_CONFIG_PATHS[version]
        if num_servers == 3:
            print("[*] Updating JSON Addresses field for 3 servers")
            update_json_addresses(version, THREE_NODEJSON_PATH, server_3node, ports_3node, is_log_experiment)
            print("[*] Broadcasting updated config...")
            broadcast_file(THREE_NODEJSON_PATH, server_3node, REMOTE_USER, remote_path, SSH_KEY_PATH)
        elif num_servers == 5:
            print("[*] Updating JSON Addresses field for 5 servers")
            update_json_addresses(version, FIVE_NODEJSON_PATH, server_5node, ports_5node, is_log_experiment)
            print("[*] Broadcasting updated config...")
            broadcast_file(FIVE_NODEJSON_PATH, server_5node, REMOTE_USER, remote_path, SSH_KEY_PATH)

def send_iptables_commands(ip, cmd):
    ssh_cmd = f'ssh -i {SSH_KEY_PATH} -o StrictHostKeyChecking=no {REMOTE_USER}@{ip} "{cmd}"'
    result = subprocess.run(ssh_cmd, shell=True, timeout=10)
    if result.returncode != 0:
        print(f"[!] Failed: {cmd} on {ip}")

def generate_iptables_commands(ip, targets, servers):
    for j in targets:
        cmd = IPTABLES_DROP_COMMAND + servers[j]
        send_iptables_commands(ip, cmd)
    cmd = IPTABLES_ACCEPT_COMMAND
    send_iptables_commands(ip, cmd)

def apply_iptables_rules(partition_type, servers):
    if partition_type == 1 or partition_type == 2:
        node_id = 0
        while node_id < len(servers) - 1:
            ip = servers[node_id]
            targets = []
            for target in range(len(servers) - 1):
                if target == node_id:
                    continue
                targets.append(target)
            send_iptables_commands(ip, targets, servers)
            node_id += 1
    elif partition_type == 3:
        node_id = 0
        while node_id < len(servers) - 3:
            ip = servers[node_id]
            targets = []
            for target in range(len(servers) - 1):
                if target == node_id:
                    continue
                targets.append(target)
            send_iptables_commands(ip, targets, servers)
            node_id += 1
        # for node_id 2
        ip = servers[node_id]
        targets = [0, 1]
        send_iptables_commands(ip, targets, servers)
        node_id += 1
        # for node_id 3
        ip = servers[node_id]
        targets = [0, 1, 4]
        send_iptables_commands(ip, targets, servers)
    elif partition_type == 4:
        node_id = 0
        while node_id < len(servers) - 2:
            ip = servers[node_id]
            targets = []
            for target in range(len(servers) - 2):
                if target == node_id:
                    continue
                targets.append(target)
            send_iptables_commands(ip, targets, servers)
            node_id += 1
        # node_id 3
        send_iptables_commands(servers[3], [4], servers)
        # node_id 4
        send_iptables_commands(servers[4], [3], servers)

def run_remote_command(ip, remote_cmd):
    ssh_cmd = [
        "ssh", "-i", SSH_KEY_PATH, "-o", "StrictHostKeyChecking=no",
        f"{REMOTE_USER}@{ip}", remote_cmd
    ]
    try:
        subprocess.run(ssh_cmd, shell=True, timeout=300)
    except Exception as e:
        print(f"[!] Error running remote command on {ip}: {e}")


def run_partition_experiment():
    parition_types = [1, 2, 3, 4]
    versions = ["holipaxos", "multipaxos", "omnipaxos", "raft"]
    for partition_type in parition_types:
        if partition_type == 2:
            servers = server_3node
            setup_config(versions, 3, servers)
        else:
            servers = server_5node
            setup_config(versions, 5, servers)
        apply_iptables_rules(partition_type)
        for version in versions:
            threads = []
            server_cmd = SERVER_CMDS[version]
            for index, ip in enumerate(servers):
                server_cmd += f"{index} partition{partition_type}"
                thread = threading.Thread(
                    target=run_remote_command,
                    args=(ip, server_cmd),
                    daemon=True
                )
                thread.start()
                threads.append(thread)
            time.sleep(10)

            client_cmd = YCSB_CLIENT_CMD + f"{version}_partition{partition_type}"
            client_thread = threading.Thread(
                target=run_remote_command,
                args=(client_machine_ip, client_cmd),
                daemon=True
            )
            client_thread.start()

            time.sleep(80)
            for index, ip in enumerate(servers):
                if index == len(servers) - 1:
                    break
                try:
                    ssh_cmd = f'ssh -i {SSH_KEY_PATH} -o StrictHostKeyChecking=no {REMOTE_USER}@{ip} "{IPTABLES_DELETE_COMMAND}"'
                    subprocess.run(ssh_cmd, shell=True)
                except Exception as e:
                    print(f"[!] Error running remote command on {ip}: {e}")
            time.sleep(20)
            for index, ip in enumerate(servers):
                if index == len(servers) - 1:
                    break
                try:
                    ssh_cmd = f'ssh -i {SSH_KEY_PATH} -o StrictHostKeyChecking=no {REMOTE_USER}@{ip} "{IPTABLES_ACCEPT_COMMAND}"'
                    subprocess.run(ssh_cmd, shell=True)
                except Exception as e:
                    print(f"[!] Error running remote command on {ip}: {e}")
            
            client_thread.join()
            for thread in threads:
                thread.join(1)

def run_log_experiment():
    versions = ["holipaxos", "multipaxos", "omnipaxos", "raft"]
    for version in versions:
        threads = []
        server_cmd = SERVER_CMDS[version]
        for index, ip in enumerate(server_3node):
            server_cmd += f"{index} log_management"
            thread = threading.Thread(
                target=run_remote_command,
                args=(ip, server_cmd),
                daemon=True
            )
            thread.start()
            threads.append(thread)
        time.sleep(10)

        client_cmd = YCSB_CLIENT_CMD + f"{index} log_management"
        client_thread = threading.Thread(
            target=run_remote_command,
            args=(client_machine_ip, client_cmd),
            daemon=True
        )
        client_thread.start()
        time.sleep(180)
        client_thread.join()
        for thread in threads:
            thread.join(1)

def run_rapid_slowdown_experiment():
    versions = ["holipaxos", "copilot", "raft"]
    for version in versions:
        threads = []
        server_cmd = SERVER_CMDS[version]
        for index, ip in enumerate(server_3node):
            server_cmd += f"{index} rapid_slowdown"
            thread = threading.Thread(
                target=run_remote_command,
                args=(ip, server_cmd),
                daemon=True
            )
            thread.start()
            threads.append(thread)
        time.sleep(10)

        client_cmd = YCSB_CLIENT_CMD + f"{index} rapid_slowdown"
        client_thread = threading.Thread(
            target=run_remote_command,
            args=(client_machine_ip, client_cmd),
            daemon=True
        )
        client_thread.start()
        time.sleep(80)
        leader_ip = server_3node[0]
        for i in [4, 5, 6, 7]:
            cmd = f'echo 0 | sudo tee /sys/devices/system/cpu/cpu"{i}"/online'
            ssh_cmd = f'ssh -i {SSH_KEY_PATH} -o StrictHostKeyChecking=no {REMOTE_USER}@{leader_ip} "{cmd}"'
            try:
                subprocess.run(ssh_cmd, shell=True)
            except Exception as e:
                print(f"[!] Error running remote command on {ip}: {e}")

        client_thread.join()
        for thread in threads:
            thread.join(1)
        for i in [4, 5, 6, 7]:
            cmd = f'echo 1 | sudo tee /sys/devices/system/cpu/cpu"{i}"/online'
            ssh_cmd = f'ssh -i {SSH_KEY_PATH} -o StrictHostKeyChecking=no {REMOTE_USER}@{leader_ip} "{cmd}"'
            try:
                subprocess.run(ssh_cmd, shell=True)
            except Exception as e:
                print(f"[!] Error running remote command on {ip}: {e}")

def main():
    print("[*] Reading IP addresses...")
    ips = read_ips(IP_LIST_PATH)
    global client_machine_ip
    global server_3node
    global server_5node
    client_machine_ip = ips[0]
    server_3node = ips[1:4]
    server_5node = ips[1:6]

    run_partition_experiment()
    run_log_experiment()
    run_rapid_slowdown_experiment()

    print("Done.")

if __name__ == "__main__":
    main()
