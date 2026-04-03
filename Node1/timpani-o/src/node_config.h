#ifndef NODE_CONFIG_H
#define NODE_CONFIG_H

#include <string>
#include <vector>
#include <map>
#include <memory>

/**
 * @brief Node configuration structure
 *
 * Contains hardware specifications and available resources for each node
 */
struct NodeConfig {
    std::string name;
    std::vector<int> available_cpus;
    int max_memory_mb;
    std::string architecture;
    std::string location;
    std::string description;

    NodeConfig() : max_memory_mb(4096) {}
};

/**
 * @brief Node configuration manager
 *
 * Handles loading and parsing YAML node configuration files
 */
class NodeConfigManager {
public:
    NodeConfigManager();
    ~NodeConfigManager();

    /**
     * @brief Load node configuration from YAML file
     * @param config_file Path to YAML configuration file
     * @return true if successfully loaded, false otherwise
     */
    bool LoadFromFile(const std::string& config_file);

    /**
     * @brief Get node configuration by name
     * @param node_name Name of the node
     * @return Pointer to NodeConfig if found, nullptr otherwise
     */
    const NodeConfig* GetNodeConfig(const std::string& node_name) const;

    /**
     * @brief Get all node configurations
     * @return Map of node name to NodeConfig
     */
    const std::map<std::string, NodeConfig>& GetAllNodes() const;

    /**
     * @brief Get available CPUs for a specific node
     * @param node_name Name of the node
     * @return Vector of available CPU IDs
     */
    std::vector<int> GetAvailableCpus(const std::string& node_name) const;

    /**
     * @brief Check if configuration is loaded
     * @return true if configuration is loaded, false otherwise
     */
    bool IsLoaded() const;

    /**
     * @brief Get default node configuration (fallback)
     * @return Default node configuration
     */
    static NodeConfig GetDefaultNodeConfig();

private:
    bool ParseYamlFile(const std::string& config_file);
    bool ParseNodeSection(const std::string& yaml_content);
    std::vector<int> ParseCpuList(const std::string& cpu_str);

    std::map<std::string, NodeConfig> nodes_;
    bool loaded_;
};

#endif // NODE_CONFIG_H
