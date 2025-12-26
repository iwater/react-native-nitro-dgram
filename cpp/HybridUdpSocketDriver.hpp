#pragma once

#include "../nitrogen/generated/shared/c++/HybridUdpSocketDriverSpec.hpp"
#include "UdpBindings.hpp"
#include <NitroModules/ArrayBuffer.hpp>
#include <string>

namespace margelo::nitro::udp {

using namespace margelo::nitro;

class HybridUdpSocketDriver : public HybridUdpSocketDriverSpec {
public:
  HybridUdpSocketDriver();
  explicit HybridUdpSocketDriver(uint32_t id);
  ~HybridUdpSocketDriver() override;

  // Properties
  double getId() override;
  std::function<void(const std::shared_ptr<ArrayBuffer> &, const std::string &,
                     double)>
  getOnMessage() override;
  void setOnMessage(
      const std::function<void(const std::shared_ptr<ArrayBuffer> &,
                               const std::string &, double)> &onMessage)
      override;
  std::function<void(const std::string &)> getOnError() override;
  void
  setOnError(const std::function<void(const std::string &)> &onError) override;
  std::function<void()> getOnClose() override;
  void setOnClose(const std::function<void()> &onClose) override;

  // Methods
  double bind(double port, const std::string &address, bool ipv6Only) override;
  double connect(double port, const std::string &address) override;
  double disconnect() override;
  double send(const std::shared_ptr<ArrayBuffer> &data, double port,
              const std::string &address) override;
  double sendMultiple(const std::vector<std::shared_ptr<ArrayBuffer>> &data,
                      double port, const std::string &address) override;
  void close() override;
  double setBroadcast(bool flag) override;
  double setTTL(double ttl) override;
  double setMulticastTTL(double ttl) override;
  double setMulticastLoopback(bool flag) override;
  double setMulticastInterface(const std::string &interfaceAddress) override;
  double
  addMembership(const std::string &multicastAddress,
                const std::optional<std::string> &interfaceAddress) override;
  double
  dropMembership(const std::string &multicastAddress,
                 const std::optional<std::string> &interfaceAddress) override;
  double addSourceSpecificMembership(
      const std::string &sourceAddress, const std::string &groupAddress,
      const std::optional<std::string> &interfaceAddress) override;
  double dropSourceSpecificMembership(
      const std::string &sourceAddress, const std::string &groupAddress,
      const std::optional<std::string> &interfaceAddress) override;
  std::string getLocalAddress() override;
  double getLocalPort() override;
  std::string getRemoteAddress() override;
  double getRemotePort() override;

  double getRecvBufferSize() override;
  double setRecvBufferSize(double size) override;
  double getSendBufferSize() override;
  double setSendBufferSize(double size) override;
  double getSendQueueCount() override;
  double getSendQueueSize() override;

  std::function<void()> getOnConnect() override;
  void setOnConnect(const std::function<void()> &onConnect) override;

private:
  void onNativeEvent(int type, const uint8_t *data, size_t len,
                     const std::string &address, int port);

  uint32_t _id;
  uint32_t _originalId; // Preserved for handler cleanup after close()
  std::function<void(const std::shared_ptr<ArrayBuffer> &, const std::string &,
                     double)>
      _onMessage;
  std::function<void()> _onConnect;
  std::function<void(const std::string &)> _onError;
  std::function<void()> _onClose;
};

} // namespace margelo::nitro::udp
