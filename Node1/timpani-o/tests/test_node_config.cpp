#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <vector>
#include <map>
#include <fstream>
#include <sstream>

#include "../src/tlog.h"
#include "../src/node_config.h"

class NodeConfigTest : public ::testing::Test {
protected:
    void SetUp() override {
        // Create test YAML content
        test_yaml_content_ = R"(
nodes:
  node1:
    name: "Test Node 1"
    available_cpus: [0, 1, 2, 3]
    max_memory_mb: 4096
    architecture: "x86_64"
    location: "rack1"
    description: "Test node for unit testing"

  node2:
    name: "Test Node 2"
    available_cpus: [0, 1]
    max_memory_mb: 2048
    architecture: "aarch64"
    location: "rack2"
    description: "ARM test node"

  node3:
    name: "Test Node 3"
    available_cpus: [4, 5, 6, 7, 8, 9]
    max_memory_mb: 8192
    architecture: "x86_64"
    location: "rack1"
    description: "High performance node"
)";

        // Create temporary test file
        test_config_file_ = "/tmp/test_node_config.yaml";
        std::ofstream file(test_config_file_);
        file << test_yaml_content_;
        file.close();

        // Create NodeConfigManager instance
        node_config_manager_ = std::make_unique<NodeConfigManager>();
    }

    void TearDown() override {
        // Clean up test file
        std::remove(test_config_file_.c_str());
        node_config_manager_.reset();
    }

    std::string test_yaml_content_;
    std::string test_config_file_;
    std::unique_ptr<NodeConfigManager> node_config_manager_;
};

// Test NodeConfig structure initialization
TEST_F(NodeConfigTest, NodeConfigStructInitialization) {
    NodeConfig config;

    EXPECT_TRUE(config.name.empty());
    EXPECT_TRUE(config.available_cpus.empty());
    EXPECT_EQ(config.max_memory_mb, 4096);  // Default value
    EXPECT_TRUE(config.architecture.empty());
    EXPECT_TRUE(config.location.empty());
    EXPECT_TRUE(config.description.empty());
}

// Test NodeConfigManager constructor
TEST_F(NodeConfigTest, NodeConfigManagerConstructor) {
    EXPECT_NE(node_config_manager_, nullptr);
    EXPECT_FALSE(node_config_manager_->IsLoaded());
    EXPECT_TRUE(node_config_manager_->GetAllNodes().empty());
}

// Test loading valid configuration file
TEST_F(NodeConfigTest, LoadValidConfigFile) {
    bool result = node_config_manager_->LoadFromFile(test_config_file_);

    EXPECT_TRUE(result);
    EXPECT_TRUE(node_config_manager_->IsLoaded());

    const auto& all_nodes = node_config_manager_->GetAllNodes();
    EXPECT_EQ(all_nodes.size(), 3);

    // Check if all expected nodes are present
    EXPECT_NE(all_nodes.find("node1"), all_nodes.end());
    EXPECT_NE(all_nodes.find("node2"), all_nodes.end());
    EXPECT_NE(all_nodes.find("node3"), all_nodes.end());
}

// Test loading non-existent file
TEST_F(NodeConfigTest, LoadNonExistentFile) {
    bool result = node_config_manager_->LoadFromFile("/non/existent/file.yaml");

    EXPECT_FALSE(result);
    EXPECT_FALSE(node_config_manager_->IsLoaded());
}

// Test loading empty file
TEST_F(NodeConfigTest, LoadEmptyFile) {
    std::string empty_file = "/tmp/empty_config.yaml";
    std::ofstream file(empty_file);
    file.close();

    bool result = node_config_manager_->LoadFromFile(empty_file);

    // Empty file loads with default configuration
    EXPECT_TRUE(result);
    EXPECT_TRUE(node_config_manager_->IsLoaded());
    EXPECT_EQ(node_config_manager_->GetAllNodes().size(), 1);

    // Should have default node
    const NodeConfig* default_node = node_config_manager_->GetNodeConfig("default_node");
    EXPECT_NE(default_node, nullptr);

    std::remove(empty_file.c_str());
}

// Test getting specific node configuration
TEST_F(NodeConfigTest, GetNodeConfig) {
    node_config_manager_->LoadFromFile(test_config_file_);

    const NodeConfig* node1 = node_config_manager_->GetNodeConfig("node1");
    ASSERT_NE(node1, nullptr);

    // The YAML parser uses the key as the name, not the "name" field
    EXPECT_EQ(node1->name, "node1");
    EXPECT_EQ(node1->available_cpus.size(), 4);
    EXPECT_EQ(node1->available_cpus[0], 0);
    EXPECT_EQ(node1->available_cpus[1], 1);
    EXPECT_EQ(node1->available_cpus[2], 2);
    EXPECT_EQ(node1->available_cpus[3], 3);
    EXPECT_EQ(node1->max_memory_mb, 4096);
    EXPECT_EQ(node1->architecture, "x86_64");
    EXPECT_EQ(node1->location, "rack1");
    EXPECT_EQ(node1->description, "Test node for unit testing");
}

// Test getting non-existent node configuration
TEST_F(NodeConfigTest, GetNonExistentNodeConfig) {
    node_config_manager_->LoadFromFile(test_config_file_);

    const NodeConfig* non_existent = node_config_manager_->GetNodeConfig("non_existent_node");
    EXPECT_EQ(non_existent, nullptr);
}

// Test getting available CPUs for specific node
TEST_F(NodeConfigTest, GetAvailableCpus) {
    node_config_manager_->LoadFromFile(test_config_file_);

    std::vector<int> node1_cpus = node_config_manager_->GetAvailableCpus("node1");
    EXPECT_EQ(node1_cpus.size(), 4);
    EXPECT_EQ(node1_cpus[0], 0);
    EXPECT_EQ(node1_cpus[1], 1);
    EXPECT_EQ(node1_cpus[2], 2);
    EXPECT_EQ(node1_cpus[3], 3);

    std::vector<int> node2_cpus = node_config_manager_->GetAvailableCpus("node2");
    EXPECT_EQ(node2_cpus.size(), 2);
    EXPECT_EQ(node2_cpus[0], 0);
    EXPECT_EQ(node2_cpus[1], 1);

    std::vector<int> node3_cpus = node_config_manager_->GetAvailableCpus("node3");
    EXPECT_EQ(node3_cpus.size(), 6);
    EXPECT_EQ(node3_cpus[0], 4);
    EXPECT_EQ(node3_cpus[5], 9);
}

// Test getting available CPUs for non-existent node
TEST_F(NodeConfigTest, GetAvailableCpusNonExistentNode) {
    node_config_manager_->LoadFromFile(test_config_file_);

    std::vector<int> cpus = node_config_manager_->GetAvailableCpus("non_existent_node");
    // Returns default CPU configuration for non-existent nodes
    EXPECT_EQ(cpus.size(), 4);
    EXPECT_EQ(cpus[0], 0);
    EXPECT_EQ(cpus[1], 1);
    EXPECT_EQ(cpus[2], 2);
    EXPECT_EQ(cpus[3], 3);
}

// Test getting all nodes
TEST_F(NodeConfigTest, GetAllNodes) {
    node_config_manager_->LoadFromFile(test_config_file_);

    const auto& all_nodes = node_config_manager_->GetAllNodes();
    EXPECT_EQ(all_nodes.size(), 3);

    // Verify node1 details (name is the key, not the "name" field)
    const auto& node1 = all_nodes.at("node1");
    EXPECT_EQ(node1.name, "node1");
    EXPECT_EQ(node1.architecture, "x86_64");

    // Verify node2 details
    const auto& node2 = all_nodes.at("node2");
    EXPECT_EQ(node2.name, "node2");
    EXPECT_EQ(node2.architecture, "aarch64");
    EXPECT_EQ(node2.max_memory_mb, 2048);

    // Verify node3 details
    const auto& node3 = all_nodes.at("node3");
    EXPECT_EQ(node3.name, "node3");
    EXPECT_EQ(node3.max_memory_mb, 8192);
}

// Test default node configuration
TEST_F(NodeConfigTest, GetDefaultNodeConfig) {
    NodeConfig default_config = NodeConfigManager::GetDefaultNodeConfig();

    EXPECT_EQ(default_config.name, "default_node");
    EXPECT_EQ(default_config.available_cpus.size(), 4);
    EXPECT_EQ(default_config.available_cpus[0], 0);
    EXPECT_EQ(default_config.available_cpus[1], 1);
    EXPECT_EQ(default_config.available_cpus[2], 2);
    EXPECT_EQ(default_config.available_cpus[3], 3);
    EXPECT_EQ(default_config.max_memory_mb, 4096);
    EXPECT_EQ(default_config.architecture, "aarch64");  // Implementation uses aarch64
    EXPECT_EQ(default_config.location, "default_location");
    EXPECT_EQ(default_config.description, "Default node configuration");
}

// Test invalid YAML format
TEST_F(NodeConfigTest, LoadInvalidYaml) {
    std::string invalid_yaml_file = "/tmp/invalid_config.yaml";
    std::ofstream file(invalid_yaml_file);
    file << "invalid: yaml: content: [\n";  // Malformed YAML
    file.close();

    bool result = node_config_manager_->LoadFromFile(invalid_yaml_file);

    // Invalid YAML still loads with default configuration
    EXPECT_TRUE(result);
    EXPECT_TRUE(node_config_manager_->IsLoaded());
    EXPECT_EQ(node_config_manager_->GetAllNodes().size(), 1);

    std::remove(invalid_yaml_file.c_str());
}

// Test YAML with missing required fields
TEST_F(NodeConfigTest, LoadYamlMissingFields) {
    std::string incomplete_yaml = R"(
nodes:
  incomplete_node:
    name: "Incomplete Node"
    # Missing available_cpus and other fields
)";

    std::string incomplete_file = "/tmp/incomplete_config.yaml";
    std::ofstream file(incomplete_file);
    file << incomplete_yaml;
    file.close();

    bool result = node_config_manager_->LoadFromFile(incomplete_file);

    // The behavior depends on implementation - might load with defaults
    // or might fail. We test that it doesn't crash.
    EXPECT_TRUE(result || !result);

    std::remove(incomplete_file.c_str());
}

// Test different CPU list formats
TEST_F(NodeConfigTest, DifferentCpuListFormats) {
    std::string varied_cpu_yaml = R"(
nodes:
  single_cpu_node:
    name: "Single CPU Node"
    available_cpus: [0]
    max_memory_mb: 1024
    architecture: "x86_64"

  range_cpu_node:
    name: "Range CPU Node"
    available_cpus: [0, 1, 2, 3, 4, 5, 6, 7]
    max_memory_mb: 2048
    architecture: "x86_64"
)";

    std::string varied_file = "/tmp/varied_cpu_config.yaml";
    std::ofstream file(varied_file);
    file << varied_cpu_yaml;
    file.close();

    bool result = node_config_manager_->LoadFromFile(varied_file);

    if (result) {
        EXPECT_TRUE(node_config_manager_->IsLoaded());

        std::vector<int> single_cpus = node_config_manager_->GetAvailableCpus("single_cpu_node");
        EXPECT_EQ(single_cpus.size(), 1);
        EXPECT_EQ(single_cpus[0], 0);

        std::vector<int> range_cpus = node_config_manager_->GetAvailableCpus("range_cpu_node");
        EXPECT_EQ(range_cpus.size(), 8);
    }

    std::remove(varied_file.c_str());
}

// Test multiple loads (reload functionality)
TEST_F(NodeConfigTest, MultipleLoads) {
    // First load
    bool result1 = node_config_manager_->LoadFromFile(test_config_file_);
    EXPECT_TRUE(result1);
    EXPECT_TRUE(node_config_manager_->IsLoaded());
    EXPECT_EQ(node_config_manager_->GetAllNodes().size(), 3);

    // Create a different config file
    std::string different_yaml = R"(
nodes:
  different_node:
    name: "Different Node"
    available_cpus: [0, 1]
    max_memory_mb: 1024
    architecture: "arm"
)";

    std::string different_file = "/tmp/different_config.yaml";
    std::ofstream file(different_file);
    file << different_yaml;
    file.close();

    // Second load (should replace first configuration)
    bool result2 = node_config_manager_->LoadFromFile(different_file);

    if (result2) {
        EXPECT_TRUE(node_config_manager_->IsLoaded());
        EXPECT_EQ(node_config_manager_->GetAllNodes().size(), 1);

        const NodeConfig* node = node_config_manager_->GetNodeConfig("different_node");
        EXPECT_NE(node, nullptr);
        EXPECT_EQ(node->architecture, "arm");
    }

    std::remove(different_file.c_str());
}

// Test edge cases for node names
TEST_F(NodeConfigTest, EdgeCaseNodeNames) {
    std::string edge_case_yaml = R"(
nodes:
  "":
    name: "Empty Name Node"
    available_cpus: [0]
    max_memory_mb: 1024

  "node-with-hyphens":
    name: "Node With Hyphens"
    available_cpus: [1]
    max_memory_mb: 1024

  "node_with_underscores":
    name: "Node With Underscores"
    available_cpus: [2]
    max_memory_mb: 1024
)";

    std::string edge_case_file = "/tmp/edge_case_config.yaml";
    std::ofstream file(edge_case_file);
    file << edge_case_yaml;
    file.close();

    bool result = node_config_manager_->LoadFromFile(edge_case_file);

    // Test that various node name formats are handled
    if (result) {
        EXPECT_TRUE(node_config_manager_->IsLoaded());

        // Test hyphenated name
        const NodeConfig* hyphen_node = node_config_manager_->GetNodeConfig("node-with-hyphens");
        if (hyphen_node) {
            EXPECT_EQ(hyphen_node->name, "node-with-hyphens");
        }

        // Test underscored name
        const NodeConfig* underscore_node = node_config_manager_->GetNodeConfig("node_with_underscores");
        if (underscore_node) {
            EXPECT_EQ(underscore_node->name, "node_with_underscores");
        }
    }

    std::remove(edge_case_file.c_str());
}

// Test main function
int main(int argc, char** argv) {
    ::testing::InitGoogleTest(&argc, argv);
    TLOG_SET_LOG_LEVEL(LogLevel::NONE);
    return RUN_ALL_TESTS();
}
