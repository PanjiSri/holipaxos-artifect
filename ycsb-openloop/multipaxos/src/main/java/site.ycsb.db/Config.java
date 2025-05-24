package site.ycsb.db;

import org.codehaus.jackson.annotate.JsonProperty;

import java.util.List;

/**
 *
 */
public class Config {
  @JsonProperty("LeaderID")
  private int leaderId;
  @JsonProperty("Address")
  private List<String> serverAddresses;

  public int getLeaderId() {
    return leaderId;
  }

  public String getServerAddress(int id) {
    return serverAddresses.get(id);
  }

  public String getLeaderAddress() {
    return serverAddresses.get(leaderId);
  }

  public List<String> getAllServerAddresses() {
    return serverAddresses;
  }

  public int getServerCounts() {
    return serverAddresses.size();
  }
}
