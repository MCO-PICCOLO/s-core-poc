#include "node_config.h"
#include "tlog.h"
#include <fstream>
#include <sstream>
#include <algorithm>
#include <regex>

NodeConfigManager::NodeConfigManager() : loaded_(false) {
}

NodeConfigManager::~NodeConfigManager() {
}

bool NodeConfigManager::LoadFromFile(const std::string& config_file) {
    TLOG_INFO("Loading node configuration from: ", config_file);

    nodes_.clear();
    loaded_ = false;

    if (!ParseYamlFile(config_file)) {
        TLOG_ERROR("Failed to parse YAML file: ", config_file);
        return false;
    }

    if (nodes_.empty()) {
        TLOG_WARN("No nodes found in configuration file, using default configuration");
        // Add default node configuration
        NodeConfig default_node = GetDefaultNodeConfig();
        default_node.name = "default_node";
        nodes_["default_node"] = default_node;
    }

    loaded_ = true;

    TLOG_INFO("Successfully loaded ", nodes_.size(), " node configurations:");
    for (const auto& pair : nodes_) {
        const NodeConfig& node = pair.second;
        TLOG_INFO("  Node: ", node.name,
                 " | CPUs: ", node.available_cpus.size(),
                 " | Memory: ", node.max_memory_mb, "MB",
                 " | Arch: ", node.architecture);

        std::stringstream cpu_list;
        for (size_t i = 0; i < node.available_cpus.size(); i++) {
            if (i > 0) cpu_list << ", ";
            cpu_list << node.available_cpus[i];
        }
        TLOG_DEBUG("    Available CPUs: [", cpu_list.str(), "]");
    }

    return true;
}

bool NodeConfigManager::ParseYamlFile(const std::string& config_file) {
    std::ifstream file(config_file);
    if (!file.is_open()) {
        TLOG_ERROR("Cannot open configuration file: ", config_file);
        return false;
    }

    std::stringstream buffer;
    buffer << file.rdbuf();
    std::string content = buffer.str();
    file.close();

    return ParseNodeSection(content);
}

bool NodeConfigManager::ParseNodeSection(const std::string& yaml_content) {
    std::istringstream stream(yaml_content);
    std::string line;
    std::string current_node;
    NodeConfig current_config;
    bool in_nodes_section = false;
    bool in_node_definition = false;

    TLOG_DEBUG("Starting YAML parsing...");

    while (std::getline(stream, line)) {
        // Remove leading/trailing whitespace
        std::string original_line = line;
        line.erase(0, line.find_first_not_of(" \t"));
        line.erase(line.find_last_not_of(" \t") + 1);

        // Skip empty lines and comments
        if (line.empty() || line[0] == '#') {
            continue;
        }

        TLOG_DEBUG("Processing line: '", line, "'");

        // Check for nodes section
        if (line == "nodes:") {
            in_nodes_section = true;
            TLOG_DEBUG("Found nodes section");
            continue;
        }

        if (!in_nodes_section) {
            continue;
        }

        // Check for node definition (starts with two spaces + node name + colon)
        if (original_line.find("  ") == 0 && line.find(":") != std::string::npos &&
            original_line.find("    ") != 0) {
            // Save previous node if exists
            if (in_node_definition && !current_node.empty()) {
                current_config.name = current_node;
                nodes_[current_node] = current_config;
                TLOG_DEBUG("Saved node: ", current_node, " with ", current_config.available_cpus.size(), " CPUs");
            }

            // Start new node
            current_node = line.substr(0, line.find(":"));
            current_config = NodeConfig();
            in_node_definition = true;
            TLOG_DEBUG("Starting new node: ", current_node);
            continue;
        }

        if (!in_node_definition) {
            continue;
        }

        // Parse node properties (starts with four spaces)
        if (original_line.find("    ") == 0) {
            std::string property_line = line;
            size_t colon_pos = property_line.find(":");
            if (colon_pos == std::string::npos) {
                continue;
            }

            std::string key = property_line.substr(0, colon_pos);
            std::string value = property_line.substr(colon_pos + 1);

            // Remove leading/trailing whitespace from value
            value.erase(0, value.find_first_not_of(" \t"));
            value.erase(value.find_last_not_of(" \t") + 1);

            // Remove quotes if present
            if ((value.front() == '"' && value.back() == '"') ||
                (value.front() == '\'' && value.back() == '\'')) {
                value = value.substr(1, value.length() - 2);
            }

            TLOG_DEBUG("  Property: ", key, " = ", value);

            // Parse specific properties
            if (key == "available_cpus") {
                current_config.available_cpus = ParseCpuList(value);
                TLOG_DEBUG("    Parsed ", current_config.available_cpus.size(), " CPUs");
            } else if (key == "max_memory_mb") {
                current_config.max_memory_mb = std::stoi(value);
            } else if (key == "architecture") {
                current_config.architecture = value;
            } else if (key == "location") {
                current_config.location = value;
            } else if (key == "description") {
                current_config.description = value;
            }
        }
    }

    // Save last node if exists
    if (in_node_definition && !current_node.empty()) {
        current_config.name = current_node;
        nodes_[current_node] = current_config;
        TLOG_DEBUG("Saved final node: ", current_node, " with ", current_config.available_cpus.size(), " CPUs");
    }

    TLOG_DEBUG("YAML parsing completed. Found ", nodes_.size(), " nodes");
    return true;
}

std::vector<int> NodeConfigManager::ParseCpuList(const std::string& cpu_str) {
    std::vector<int> cpus;

    // Remove brackets if present
    std::string clean_str = cpu_str;
    if (clean_str.front() == '[' && clean_str.back() == ']') {
        clean_str = clean_str.substr(1, clean_str.length() - 2);
    }

    // Split by comma
    std::istringstream stream(clean_str);
    std::string token;

    while (std::getline(stream, token, ',')) {
        // Remove whitespace
        token.erase(0, token.find_first_not_of(" \t"));
        token.erase(token.find_last_not_of(" \t") + 1);

        if (!token.empty()) {
            try {
                int cpu_id = std::stoi(token);
                cpus.push_back(cpu_id);
            } catch (const std::exception& e) {
                TLOG_WARN("Invalid CPU ID in configuration: ", token);
            }
        }
    }

    return cpus;
}

const NodeConfig* NodeConfigManager::GetNodeConfig(const std::string& node_name) const {
    auto it = nodes_.find(node_name);
    if (it != nodes_.end()) {
        return &it->second;
    }
    return nullptr;
}

const std::map<std::string, NodeConfig>& NodeConfigManager::GetAllNodes() const {
    return nodes_;
}

std::vector<int> NodeConfigManager::GetAvailableCpus(const std::string& node_name) const {
    const NodeConfig* config = GetNodeConfig(node_name);
    if (config) {
        return config->available_cpus;
    }

    // Return default CPU configuration
    return {0, 1, 2, 3};
}

bool NodeConfigManager::IsLoaded() const {
    return loaded_;
}

NodeConfig NodeConfigManager::GetDefaultNodeConfig() {
    NodeConfig config;
    config.name = "default_node";
    config.available_cpus = {0, 1, 2, 3};
    config.max_memory_mb = 4096;
    config.architecture = "aarch64";
    config.location = "default_location";
    config.description = "Default node configuration";
    return config;
}
