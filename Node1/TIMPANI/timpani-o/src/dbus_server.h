/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef DBUS_SERVER_H
#define DBUS_SERVER_H

#include <libtrpc.h>

#include <atomic>
#include <map>
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

    // Invalidate the cached serialized schedule so it is recomputed on the
    // next timpani-n pull.  Call this whenever the schedule changes.
    void FreeSchedInfoBuf();

  private:
    DBusServer();
    ~DBusServer();

    // Event loop for handling D-Bus messages
    void EventLoop();

    // Serialize scheduling information for libtrpc callbacks.
    // node_name is the requesting Timpani-N node; only its workload is serialized.
    bool SerializeSchedInfo(const SchedInfoMap& map, const std::string& node_name);

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
    // Per-node serialized schedule buffers (keyed by node name).
    // Each Timpani-N node gets its own buffer so multiple workloads on
    // different nodes can coexist without overwriting each other.
    std::map<std::string, serial_buf_t*> sched_info_buf_per_node_;
    // Mutex to protect sched_info_buf_per_node_ access
    std::mutex sched_info_buf_mutex_;
    // Map to track synchronization status of each node for SyncCallback
    std::unordered_map<std::string, bool> node_sync_map_;
};

#endif  // DBUS_SERVER_H
