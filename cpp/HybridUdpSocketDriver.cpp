#include "HybridUdpSocketDriver.hpp"
#include "UdpManager.hpp"

namespace margelo::nitro::udp {

HybridUdpSocketDriver::HybridUdpSocketDriver() : HybridObject(TAG) {
  _id = ::udp::udp_create();
  _originalId = _id;
  UdpManager::shared().registerHandler(
      _id, [this](int type, const uint8_t *data, size_t len,
                  const std::string &address, int port) {
        this->onNativeEvent(type, data, len, address, port);
      });
}

HybridUdpSocketDriver::HybridUdpSocketDriver(uint32_t id)
    : HybridObject(TAG), _id(id), _originalId(id) {
  UdpManager::shared().registerHandler(
      _id, [this](int type, const uint8_t *data, size_t len,
                  const std::string &address, int port) {
        this->onNativeEvent(type, data, len, address, port);
      });
}

HybridUdpSocketDriver::~HybridUdpSocketDriver() { close(); }

double HybridUdpSocketDriver::getId() { return static_cast<double>(_id); }

std::function<void(const std::shared_ptr<ArrayBuffer> &, const std::string &,
                   double)>
HybridUdpSocketDriver::getOnMessage() {
  return _onMessage;
}
void HybridUdpSocketDriver::setOnMessage(
    const std::function<void(const std::shared_ptr<ArrayBuffer> &,
                             const std::string &, double)> &onMessage) {
  _onMessage = onMessage;
}

std::function<void(const std::string &)> HybridUdpSocketDriver::getOnError() {
  return _onError;
}
void HybridUdpSocketDriver::setOnError(
    const std::function<void(const std::string &)> &onError) {
  _onError = onError;
}

std::function<void()> HybridUdpSocketDriver::getOnClose() { return _onClose; }
void HybridUdpSocketDriver::setOnClose(const std::function<void()> &onClose) {
  _onClose = onClose;
}

double HybridUdpSocketDriver::bind(double port, const std::string &address,
                                   bool ipv6Only) {
  return static_cast<double>(
      ::udp::udp_bind(_id, address.empty() ? nullptr : address.c_str(),
                      static_cast<int>(port), ipv6Only));
}

double HybridUdpSocketDriver::connect(double port, const std::string &address) {
  return static_cast<double>(
      ::udp::udp_connect(_id, address.c_str(), static_cast<int>(port)));
}

double HybridUdpSocketDriver::disconnect() {
  return static_cast<double>(::udp::udp_disconnect(_id));
}

double HybridUdpSocketDriver::send(const std::shared_ptr<ArrayBuffer> &data,
                                   double port, const std::string &address) {
  if (!data)
    return -1;
  return static_cast<double>(::udp::udp_send(_id, data->data(), data->size(),
                                             address.c_str(),
                                             static_cast<int>(port)));
}

double HybridUdpSocketDriver::sendMultiple(
    const std::vector<std::shared_ptr<ArrayBuffer>> &data, double port,
    const std::string &address) {
  if (data.empty())
    return 0;

  std::vector<const uint8_t *> datas;
  std::vector<size_t> lens;
  datas.reserve(data.size());
  lens.reserve(data.size());

  for (const auto &buffer : data) {
    if (buffer) {
      datas.push_back(buffer->data());
      lens.push_back(buffer->size());
    }
  }

  if (datas.empty())
    return 0;

  return static_cast<double>(
      ::udp::udp_send_multiple(_id, datas.data(), lens.data(), datas.size(),
                               address.c_str(), static_cast<int>(port)));
}

void HybridUdpSocketDriver::close() {
  if (_id != 0) {
    ::udp::udp_close(_id);
    // NOTE: Do NOT unregister handler here!
    // The handler will be unregistered when EVENT_CLOSE is received in
    // onNativeEvent. This ensures the close event can be properly delivered to
    // JS layer.
    _id = 0;
  }
}

double HybridUdpSocketDriver::setBroadcast(bool flag) {
  return static_cast<double>(::udp::udp_set_broadcast(_id, flag));
}
double HybridUdpSocketDriver::setTTL(double ttl) {
  return static_cast<double>(
      ::udp::udp_set_ttl(_id, static_cast<uint32_t>(ttl)));
}
double HybridUdpSocketDriver::setMulticastTTL(double ttl) {
  return static_cast<double>(
      ::udp::udp_set_multicast_ttl_v4(_id, static_cast<uint32_t>(ttl)));
}
double HybridUdpSocketDriver::setMulticastLoopback(bool flag) {
  return static_cast<double>(::udp::udp_set_multicast_loop_v4(_id, flag));
}
double HybridUdpSocketDriver::setMulticastInterface(
    const std::string &interfaceAddress) {
  return -1;
}
double HybridUdpSocketDriver::addMembership(
    const std::string &multicastAddress,
    const std::optional<std::string> &interfaceAddress) {
  return -1;
}
double HybridUdpSocketDriver::dropMembership(
    const std::string &multicastAddress,
    const std::optional<std::string> &interfaceAddress) {
  return -1;
}

double HybridUdpSocketDriver::addSourceSpecificMembership(
    const std::string &sourceAddress, const std::string &groupAddress,
    const std::optional<std::string> &interfaceAddress) {
  return static_cast<double>(::udp::udp_add_source_specific_membership(
      _id, sourceAddress.c_str(), groupAddress.c_str(),
      interfaceAddress.has_value() ? interfaceAddress.value().c_str()
                                   : nullptr));
}

double HybridUdpSocketDriver::dropSourceSpecificMembership(
    const std::string &sourceAddress, const std::string &groupAddress,
    const std::optional<std::string> &interfaceAddress) {
  return static_cast<double>(::udp::udp_drop_source_specific_membership(
      _id, sourceAddress.c_str(), groupAddress.c_str(),
      interfaceAddress.has_value() ? interfaceAddress.value().c_str()
                                   : nullptr));
}

std::string HybridUdpSocketDriver::getLocalAddress() {
  char buf[256];
  if (::udp::udp_get_local_addr(_id, buf, sizeof(buf)) == 0) {
    return std::string(buf);
  }
  return "";
}

double HybridUdpSocketDriver::getLocalPort() {
  return static_cast<double>(::udp::udp_get_local_port(_id));
}

std::string HybridUdpSocketDriver::getRemoteAddress() {
  char buf[256];
  if (::udp::udp_get_peer_addr(_id, buf, sizeof(buf)) == 0) {
    return std::string(buf);
  }
  return "";
}

double HybridUdpSocketDriver::getRemotePort() {
  return static_cast<double>(::udp::udp_get_peer_port(_id));
}

double HybridUdpSocketDriver::getRecvBufferSize() {
  return static_cast<double>(::udp::udp_get_buffer_size(_id, false));
}

double HybridUdpSocketDriver::setRecvBufferSize(double size) {
  return static_cast<double>(
      ::udp::udp_set_buffer_size(_id, false, static_cast<int>(size)));
}

double HybridUdpSocketDriver::getSendBufferSize() {
  return static_cast<double>(::udp::udp_get_buffer_size(_id, true));
}

double HybridUdpSocketDriver::setSendBufferSize(double size) {
  return static_cast<double>(
      ::udp::udp_set_buffer_size(_id, true, static_cast<int>(size)));
}

double HybridUdpSocketDriver::getSendQueueCount() {
  return static_cast<double>(::udp::udp_get_send_queue_count(_id));
}

double HybridUdpSocketDriver::getSendQueueSize() {
  return static_cast<double>(::udp::udp_get_send_queue_size(_id));
}

std::function<void()> HybridUdpSocketDriver::getOnConnect() {
  return _onConnect;
}
void HybridUdpSocketDriver::setOnConnect(
    const std::function<void()> &onConnect) {
  _onConnect = onConnect;
}

void HybridUdpSocketDriver::onNativeEvent(int type, const uint8_t *data,
                                          size_t len,
                                          const std::string &address,
                                          int port) {
  switch (type) {
  case 1: // DATA
    if (_onMessage) {
      auto ab = ArrayBuffer::copy(data, len);
      _onMessage(ab, address, static_cast<double>(port));
    }
    break;
  case 2: // ERROR
    if (_onError) {
      _onError(data ? std::string(reinterpret_cast<const char *>(data), len)
                    : "Unknown error");
    }
    break;
  case 3: // CLOSE
    // Unregister handler now that we've received the close event
    UdpManager::shared().unregisterHandler(_originalId);
    if (_onClose) {
      _onClose();
    }
    break;
  case 4: // CONNECT
    if (_onConnect) {
      _onConnect();
    }
    break;
  }
}

} // namespace margelo::nitro::udp
