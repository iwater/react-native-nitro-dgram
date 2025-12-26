#ifndef UDP_BINDINGS_H
#define UDP_BINDINGS_H

#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

namespace udp {

using CallbackPtr = void(*)(uint32_t id,
                            int event_type,
                            const uint8_t *data,
                            uintptr_t len,
                            const char *address,
                            int port,
                            void *context);

extern "C" {

void udp_init(CallbackPtr callback, void *context);

void udp_init_with_config(CallbackPtr callback, void *context, uint32_t worker_threads);

uint32_t udp_create();

int32_t udp_bind(uint32_t id, const char *host, int port, bool ipv6_only);

int32_t udp_send(uint32_t id, const uint8_t *data, uintptr_t len, const char *host, int port);

int32_t udp_get_send_queue_count(uint32_t id);

int32_t udp_get_send_queue_size(uint32_t id);

int32_t udp_send_multiple(uint32_t id,
                          const uint8_t *const *datas,
                          const uintptr_t *lens,
                          uintptr_t count,
                          const char *host,
                          int port);

int32_t udp_connect(uint32_t id, const char *host, int port);

int32_t udp_get_buffer_size(uint32_t id, bool is_send);

int32_t udp_set_buffer_size(uint32_t id, bool is_send, int32_t size);

int32_t udp_disconnect(uint32_t _id);

void udp_close(uint32_t id);

int32_t udp_set_broadcast(uint32_t id, bool flag);

int32_t udp_set_ttl(uint32_t id, uint32_t ttl);

int32_t udp_set_multicast_ttl_v4(uint32_t id, uint32_t ttl);

int32_t udp_set_multicast_loop_v4(uint32_t id, bool flag);

int32_t udp_get_local_addr(uint32_t id, char *buf, uintptr_t max_len);

int32_t udp_get_local_port(uint32_t id);

int32_t udp_get_peer_addr(uint32_t id, char *buf, uintptr_t max_len);

int32_t udp_get_peer_port(uint32_t id);

int32_t udp_add_source_specific_membership(uint32_t id,
                                           const char *source_addr,
                                           const char *group_addr,
                                           const char *interface_addr);

int32_t udp_drop_source_specific_membership(uint32_t id,
                                            const char *source_addr,
                                            const char *group_addr,
                                            const char *interface_addr);

} // extern "C"

} // namespace udp

#endif // UDP_BINDINGS_H
