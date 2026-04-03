#ifndef DBUS_SERVER_H
#define DBUS_SERVER_H

#include <libtrpc.h>

#include <atomic>
#include <thread>
#include <mutex>
#include <unordered_map>

#include "schedinfo_service.h"

/**
 * @brief DBusServer class for D-Bus communication with Timpani-N
 *
 * This class manages the D-Bus server, event loop, and callbacks for
 * scheduling information, deadline misses, and multi-node synchronization.
 * It uses libtrpc for remote procedure calls over D-Bus.
 *
 * Implemented as a thread-safe singleton to ensure only one D-Bus server
 * instance per process and proper resource management.
 */
class DBusServer
{
  public:
    static DBusServer& GetInstance();

    // Delete copy constructor and assignment operator
    DBusServer(const DBusServer&) = delete;
    DBusServer& operator=(const DBusServer&) = delete;

    bool Start(int port, SchedInfoServer* sinfo_server = nullptr);
    void Stop();
    void SetSchedInfoServer(SchedInfoServer* server);

  private:
    DBusServer();
    ~DBusServer();

    // Event loop for handling D-Bus messages
    void EventLoop();

    // Serialize scheduling information for libtrpc callbacks
    bool SerializeSchedInfo(const SchedInfoMap& map);
    void FreeSchedInfoBuf();

    // Static callbacks for libtrpc
    static void RegisterCallback(const char* name);
    static void SchedInfoCallback(const char* name, void** buf,
                                  size_t* bufsize);
    static void DMissCallback(const char* name, const char* task);
    static void SyncCallback(const char* name, int* ack, struct timespec* ts);

    // Member variables for D-Bus server
    sd_event_source* event_source_;
    sd_event* event_;
    std::unique_ptr<std::thread> event_thread_;
    int server_fd_;
    std::atomic<bool> running_;

    // SchedInfoServer instance for libtrpc callbacks
    SchedInfoServer* sched_info_server_;
    // Buffer to store serialized scheduling info for libtrpc callbacks
    serial_buf_t* sched_info_buf_;
    // Mutex to protect sched_info_buf_ access
    std::mutex sched_info_buf_mutex_;
    // Map to track synchronization status of each node for SyncCallback
    std::unordered_map<std::string, bool> node_sync_map_;
    // Universal sync timestamp shared across all nodes in SyncCallback
    struct timespec sync_ts_;
    // Mutex to protect sync_ts_ access
    std::mutex sync_ts_mutex_;
};

#endif  // DBUS_SERVER_H
