#pragma once

#include "../nitrogen/generated/shared/c++/HybridUdpDriverSpec.hpp"
#include <memory>
#include <optional>
#include <string>

namespace margelo::nitro::udp {

class HybridUdpDriver : public HybridUdpDriverSpec {
public:
  HybridUdpDriver();

  std::shared_ptr<HybridUdpSocketDriverSpec>
  createSocket(const std::optional<std::string> &id) override;
};

} // namespace margelo::nitro::udp
