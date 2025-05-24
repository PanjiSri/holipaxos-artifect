package site.ycsb.db;

import io.etcd.jetcd.ByteSequence;
import io.etcd.jetcd.Client;
import io.etcd.jetcd.KV;
import io.etcd.jetcd.kv.GetResponse;
import org.codehaus.jackson.map.ObjectMapper;
import site.ycsb.*;

import java.io.File;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.Set;
import java.util.Vector;
import java.util.concurrent.ExecutionException;

/**
 *
 */
public class EtcdClient extends DB {
  private KV kvClient;
  private Config config;

  @Override
  public void init() throws DBException {
    ObjectMapper objectMapper = new ObjectMapper();
    try {
      config = objectMapper.readValue(
          new File("config.json"),
          Config.class);
    } catch (IOException e) {
      System.err.println("Couldn't load config.json");
      System.exit(1);
    }
    String serverAddress = config.getLeaderAddress();
    Client client = Client.builder().endpoints("http://" + serverAddress).build();
    kvClient = client.getKVClient();
  }

  @Override
  public Status read(String table, String key, Set<String> fields, Map<String, ByteIterator> result) {
    ByteSequence keyByte = ByteSequence.from(key.getBytes());
    try {
      GetResponse response = kvClient.get(keyByte).get();
      if (response.getCount() == 0) {
        return Status.ERROR;
      }
      String value = response.getKvs().get(0).getValue().toString();
      result.put("field1", new StringByteIterator(value));
    } catch (ExecutionException | InterruptedException e) {
      e.printStackTrace();
      return Status.ERROR;
    }
    return Status.OK;
  }

  @Override
  public Status scan(String table, String startkey, int recordcount, Set<String> fields,
      Vector<HashMap<String, ByteIterator>> result) {
    return Status.NOT_IMPLEMENTED;
  }

  @Override
  public Status update(String table, String key, Map<String, ByteIterator> values) {
    return insert(table, key, values);
  }

  @Override
  public Status insert(String table, String key, Map<String, ByteIterator> values) {
    StringBuilder value = new StringBuilder();
    for (Map.Entry<String, ByteIterator> entry : values.entrySet()) {
      value.append(entry.getValue().toString());
    }
    ByteSequence keyByte = ByteSequence.from(key.getBytes());
    ByteSequence valueByte = ByteSequence.from(value.toString().getBytes());
    try {
      kvClient.put(keyByte, valueByte).get();
    } catch (ExecutionException | InterruptedException e) {
      e.printStackTrace();
      return Status.ERROR;
    }
    return Status.OK;
  }

  @Override
  public Status delete(String table, String key) {
    return Status.NOT_IMPLEMENTED;
  }
}
