#include "HybridUdpDriver.hpp"
#include "HybridUdpSocketDriver.hpp"

namespace margelo::nitro::udp {

HybridUdpDriver::HybridUdpDriver() : HybridObject(TAG) {}

std::shared_ptr<HybridUdpSocketDriverSpec>
HybridUdpDriver::createSocket(const std::optional<std::string> &id) {
  if (id.has_value()) {
    try {
      uint32_t socketId = static_cast<uint32_t>(std::stoul(id.value()));
      return std::make_shared<HybridUdpSocketDriver>(socketId);
    } catch (...) {
      // Fallback
    }
  }
  return std::make_shared<HybridUdpSocketDriver>();
}

} // namespace margelo::nitro::udp
