package site.ycsb.db;

import java.net.SocketTimeoutException;
import java.util.AbstractMap.SimpleEntry;
import java.util.Map.Entry;
import java.util.concurrent.ConcurrentLinkedQueue;
import org.codehaus.jackson.map.ObjectMapper;
import site.ycsb.*;

import java.io.*;
import java.net.Socket;
import java.util.*;
import site.ycsb.measurements.Measurements;

/**
 *
 */
public class MultipaxosClient extends DB {
  private Socket socket;
  private Config config;
  private PrintWriter writer;
  private BufferedReader reader;
  private int leaderId;
  private List<Socket> sockets;
  private List<PrintWriter> writers;
  private List<BufferedReader> readers;
  private Measurements measurements;
  private Queue<Entry<Long, Long>> queue;
  private Thread thread;

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
    leaderId = config.getLeaderId();
    sockets = new ArrayList<>();
    writers = new ArrayList<>();
    readers = new ArrayList<>();
    measurements = Measurements.getMeasurements();
    queue = new ConcurrentLinkedQueue<>();
    connect();
    thread = new Thread(this::onReceive);
    thread.start();
  }

  private void connect() {
    List<String> addresses = config.getAllServerAddresses();

    for (String address : addresses) {
      String[] tokens = address.split(":");
      String ip = tokens[0];
      int port = Integer.parseInt(tokens[1]);
      try {
        Socket s = new Socket(ip, port);
        s.setSoTimeout(5000);
        PrintWriter w = new PrintWriter(s.getOutputStream(), true);
        BufferedReader r = new BufferedReader(new InputStreamReader(s.getInputStream()));
        sockets.add(s);
        writers.add(w);
        readers.add(r);
      } catch(Exception ignored) {
        //
      }
    }
    switchServer();
  }

  private void switchServer() {
    socket = sockets.get(leaderId);
    writer = writers.get(leaderId);
    reader = readers.get(leaderId);
  }

  //Read a single record
  @Override
  public Status read(final String table, final String key, final Set<String> fields,
                     final Map<String, ByteIterator> result, long ist, long st) {
    String request = "get " + key + "\n";
    try {
      sendRequest(request, ist, st);
      return Status.OK;
    } catch (Exception e) {
      return Status.ERROR;
    }
  }

  //Perform a range scan
  @Override
  public Status scan(final String table, final String startkey, final int recordcount, final Set<String> fields,
                     final Vector<HashMap<String, ByteIterator>> result, long ist, long st) {
    return Status.NOT_IMPLEMENTED;
  }

  //Update a single record
  @Override
  public Status update(final String table, final String key, final Map<String, ByteIterator> values,
      long ist, long st) {
    return insert(table, key, values, ist, st);
  }

  //Insert a single record
  @Override
  public Status insert(final String table, final String key, final Map<String, ByteIterator> values,
      long ist, long st) {
    StringBuilder value = new StringBuilder();
    for (Map.Entry<String, ByteIterator> entry : values.entrySet()) {
      value.append(entry.getValue().toString());
    }
    String request = "put " + key + " " + value + "\n";
    try {
      sendRequest(request, ist, st);
      return Status.OK;
    } catch (Exception e) {
      return Status.ERROR;
    }
  }

  //Delete a single record
  @Override
  public Status delete(final String table, final String key, long ist, long st) {
    return Status.NOT_IMPLEMENTED;
  }

  @Override
  public void cleanup() {
    try {
      while (!queue.isEmpty()) {
        Thread.sleep(1000);
      }
      for (int i = 0; i < sockets.size(); i++) {
        sockets.get(i).close();
        readers.get(i).close();
        writers.get(i).close();
      }
      thread.join();
    } catch (IOException | InterruptedException e) {
      System.err.println(e.toString());
    }
  }

  private void sendRequest(String request, long ist, long st) throws Exception {
    queue.add(new SimpleEntry<>(ist, st));
    writer.write(request);
    writer.flush();
  }

  private void onReceive() {
    String result = "";
    boolean isOk;
    while (true) {
      try {
        result = reader.readLine();
      } catch (SocketTimeoutException e) {
        leaderId = (leaderId + 1) % config.getServerCounts();
        switchServer();
      } catch (IOException e) {
        break;
      }
      long endTimeNanos = System.nanoTime();

      isOk = true;
      if (Objects.equals(result, "retry") ||
          Objects.equals(result, "bad command")) {
        isOk = false;
      } else if (result.startsWith("leader is")) {
        String[] tokens = result.split(" ");
        leaderId = Integer.parseInt(tokens[2]);
        switchServer();
        isOk = false;
      }
      measure(result, isOk, endTimeNanos);
    }
  }

  private void measure(String result, boolean isOk, long endTimeNanos) {
    String measurementName = "OVERALL";
    if (result == null || !isOk) {
      measurementName += "-FAILED";
    }
    int duration, istduration;
    do {
      Entry<Long, Long> entry = queue.poll();
      if (entry == null) {
//      System.err.println("no elements in the queue");
        return;
      }
      duration = (int) ((endTimeNanos - entry.getValue()) / 1000);
      istduration = (int) ((endTimeNanos - entry.getValue()) / 1000);
    } while (duration > 130000);
    measurements.measure(measurementName, duration);
    measurements.measureIntended(measurementName, istduration);
  }
}
