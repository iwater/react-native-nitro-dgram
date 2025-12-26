#pragma once

#include "UdpBindings.hpp"
#include <functional>
#include <memory>
#include <shared_mutex>
#include <string>
#include <unordered_map>
#include <vector>

#define UM_TAG "UdpManager"

#ifdef __ANDROID__
#include <android/log.h>
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, UM_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN, UM_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, UM_TAG, __VA_ARGS__)
#else
#include <cstdio>
#define LOGI(...)                                                              \
  printf("[" UM_TAG "] " __VA_ARGS__);                                         \
  printf("\n")
#define LOGW(...)                                                              \
  printf("[" UM_TAG "] WARN: " __VA_ARGS__);                                   \
  printf("\n")
#define LOGE(...)                                                              \
  printf("[" UM_TAG "] ERROR: " __VA_ARGS__);                                  \
  printf("\n")
#endif

namespace margelo::nitro::udp {

class UdpManager {
public:
  using EventHandler =
      std::function<void(int eventType, const uint8_t *data, size_t len,
                         const std::string &address, int port)>;

  static UdpManager &shared() {
    static UdpManager instance;
    return instance;
  }

  UdpManager() {
    LOGI("Initializing UdpManager...");
    ::udp::udp_init(
        [](uint32_t id, int event_type, const uint8_t *data, uintptr_t len,
           const char *address, int port, void *context) {
          auto mgr = static_cast<UdpManager *>(context);
          mgr->dispatch(id, event_type, data, static_cast<size_t>(len),
                        address ? address : "", port);
        },
        this);
  }

public:
  void registerHandler(uint32_t id, EventHandler handler) {
    LOGI("Registering handler for ID %u", id);
    std::unique_lock lock(_mutex);
    _handlers[id] = handler;
  }

  void unregisterHandler(uint32_t id) {
    LOGI("Unregistering handler for ID %u", id);
    std::unique_lock lock(_mutex);
    _handlers.erase(id);
  }

private:
  void dispatch(uint32_t id, int eventType, const uint8_t *data, size_t len,
                const std::string &address, int port) {
    const char *eventName = "UNKNOWN";
    switch (eventType) {
    case 1:
      eventName = "DATA";
      break;
    case 2:
      eventName = "ERROR";
      break;
    case 3:
      eventName = "CLOSE";
      break;
    }

    LOGI("dispatch: id=%u, event=%s(%d), len=%zu, addr=%s:%d", id, eventName,
         eventType, len, address.c_str(), port);

    EventHandler handler;
    {
      std::shared_lock lock(_mutex);
      auto it = _handlers.find(id);
      if (it != _handlers.end()) {
        handler = it->second;
      }
    }

    if (handler) {
      handler(eventType, data, len, address, port);
    } else {
      LOGW("No handler found for id=%u, event=%s", id, eventName);
    }
  }

  std::shared_mutex _mutex;
  std::unordered_map<uint32_t, EventHandler> _handlers;
};

} // namespace margelo::nitro::udp
