#pragma once

#include "UdpBindings.hpp"
#include <functional>
#include <memory>
#include <shared_mutex>
#include <string>
#include <unordered_map>
#include <vector>

#define UM_TAG "UdpManager"

// 诊断输出开关（**默认关闭**）。
//
// 非 Android 下这三条宏原先**无条件 `printf` 到 stdout**。而 qjs-node 的 parity 判据是
// stdout/stderr/退出码与 Node 逐字节一致，Node 在 require('dgram') 这条路径上什么都不打
// —— 于是任何用到 udp 的程序必然判红。**只改成 stderr 也没用**，parity 同样比对 stderr。
// 需要排查时在编译期定义 UDPMANAGER_VERBOSE 重新打开（输出仍走 stdout，仅供本地调试）。
#ifdef UDPMANAGER_VERBOSE
#include <cstdio>
#endif

#ifdef __ANDROID__
#include <android/log.h>
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, UM_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN, UM_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, UM_TAG, __VA_ARGS__)
#elif defined(UDPMANAGER_VERBOSE)
#define LOGI(...)                                                              \
  printf("[" UM_TAG "] " __VA_ARGS__);                                         \
  printf("\n")
#define LOGW(...)                                                              \
  printf("[" UM_TAG "] WARN: " __VA_ARGS__);                                   \
  printf("\n")
#define LOGE(...)                                                              \
  printf("[" UM_TAG "] ERROR: " __VA_ARGS__);                                  \
  printf("\n")
#else
// 默认静默。仍把参数「吃掉」，否则静默后只被日志用到的变量（本文件里的 eventName）
// 会触发 -Wunused-but-set-variable。logSink 不产生任何输出。
namespace margelo::nitro::udp::detail {
template <typename... Args>
inline void logSink(const char *, Args &&...) noexcept {}
} // namespace margelo::nitro::udp::detail
#define LOGI(...) ::margelo::nitro::udp::detail::logSink(__VA_ARGS__)
#define LOGW(...) ::margelo::nitro::udp::detail::logSink(__VA_ARGS__)
#define LOGE(...) ::margelo::nitro::udp::detail::logSink(__VA_ARGS__)
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
