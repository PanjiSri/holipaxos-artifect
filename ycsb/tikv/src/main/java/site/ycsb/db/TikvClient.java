package site.ycsb.db;

import java.util.Optional;
import org.tikv.common.TiConfiguration;
import org.tikv.common.TiSession;
import org.tikv.raw.RawKVClient;
import org.tikv.shade.com.google.protobuf.ByteString;
import org.codehaus.jackson.map.ObjectMapper;
import site.ycsb.*;

import java.io.File;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.Set;
import java.util.Vector;

/**
 *
 */
public class TikvClient extends DB {
  private RawKVClient kvClient;
  private TiSession session;
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
    TiConfiguration conf = TiConfiguration.createRawDefault(config.getLeaderAddress());
    session = TiSession.create(conf);
    kvClient = session.createRawClient();
  }

  @Override
  public Status read(String table, String key, Set<String> fields, Map<String, ByteIterator> result) {
    Optional<ByteString> response = kvClient.get(ByteString.copyFromUtf8(key));
    if (!response.isPresent()) {
      return Status.ERROR;
    }
    String value = response.get().toStringUtf8();
    result.put("field1", new StringByteIterator(value));
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
    kvClient.put(ByteString.copyFromUtf8(key), ByteString.copyFromUtf8(value.toString()));
    return Status.OK;
  }

  @Override
  public Status delete(String table, String key) {
    return Status.NOT_IMPLEMENTED;
  }

  @Override
  public void cleanup() throws DBException {
    kvClient.close();
    try {
      session.close();
    } catch (Exception e) {
      e.printStackTrace();
    }
  }
}
