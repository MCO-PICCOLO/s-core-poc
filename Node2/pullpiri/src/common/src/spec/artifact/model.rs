/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use super::Artifact;
use super::Model;

pub type ModelSpec = crate::spec::k8s::pod::PodSpec;

impl Artifact for Model {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Model {
    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }

    pub fn get_podspec(&self) -> ModelSpec {
        self.spec.clone()
    }

    /// Returns a mutable reference to the PodSpec so callers can set fields such as
    /// `volumes` without losing any other spec data (e.g. `probeConfig`).
    pub fn get_podspec_mut(&mut self) -> &mut ModelSpec {
        &mut self.spec
    }
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // Test Model creation through JSON deserialization (public interface)
    fn create_test_model(name: &str) -> Model {
        let model_json = format!(
            r#"{{
                "apiVersion": "v1",
                "kind": "Model",
                "metadata": {{
                    "name": "{}"
                }},
                "spec": {{
                    "containers": [{{
                        "name": "test-container",
                        "image": "test-image"
                    }}]
                }}
            }}"#,
            name
        );

        serde_json::from_str(&model_json).unwrap()
    }

    #[test]
    fn test_get_name_via_artifact_trait() {
        let model_name = "test-model";
        let model = create_test_model(model_name);
        assert_eq!(model.get_name(), model_name);
    }

    #[test]
    fn test_get_name_via_direct_method() {
        let model_name = "test-model-direct";
        let model = create_test_model(model_name);
        assert_eq!(model.get_name(), model_name);
    }

    #[test]
    fn test_get_name_with_empty_string() {
        let model = create_test_model("");
        assert_eq!(model.get_name(), "");
    }

    #[test]
    fn test_get_name_with_special_characters() {
        let model_name = "model@123#test";
        let model = create_test_model(model_name);
        assert_eq!(model.get_name(), model_name);
    }

    #[test]
    fn test_get_podspec_returns_valid_spec() {
        let model = create_test_model("test-podspec");
        let _pod_spec = model.get_podspec(); // Prefix with underscore to silence warning
    }

    #[test]
    fn test_get_podspec_returns_clone() {
        let model = create_test_model("test-clone");
        let _pod_spec1 = model.get_podspec(); // Prefix with underscore
        let _pod_spec2 = model.get_podspec(); // Prefix with underscore
    }

    #[test]
    fn test_model_conversion_preserves_name() {
        let model_name = "conversion-test";
        let model = create_test_model(model_name);
        let pod_name = model.get_name();

        assert_eq!(pod_name, model_name);
    }

    #[test]
    fn test_get_podspec_mut_allows_modification() {
        // Verify that modifying the spec via get_podspec_mut() is reflected when
        // get_podspec() is called afterwards (unlike get_podspec() which returns a clone).
        let mut model = create_test_model("mut-test");

        // Initially volumes should be None
        assert!(model.get_podspec().volumes.is_none());

        // Modify volumes via mutable reference
        model.get_podspec_mut().volumes = Some(vec![]);

        // The modification must be visible through both get_podspec() and get_podspec_mut()
        assert!(model.get_podspec().volumes.is_some());
        assert_eq!(model.get_podspec().volumes.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_get_podspec_mut_preserves_probe_config() {
        // Modifying volumes via get_podspec_mut() must NOT lose probeConfig.
        let model_json = r#"{
            "apiVersion": "v1",
            "kind": "Model",
            "metadata": { "name": "probe-model" },
            "spec": {
                "containers": [{ "name": "c", "image": "img" }],
                "probeConfig": {
                    "liveness": {
                        "http": { "path": "/health", "port": 8080 }
                    }
                }
            }
        }"#;
        let mut model: super::Model =
            serde_json::from_str(model_json).expect("Failed to parse model");

        // Confirm probeConfig is present before mutation
        assert!(model.get_podspec().probeConfig.is_some());

        // Mutate volumes only
        model.get_podspec_mut().volumes = Some(vec![]);

        // probeConfig must still be present after mutation
        assert!(model.get_podspec().probeConfig.is_some());
        let podspec = model.get_podspec();
        let liveness = podspec
            .probeConfig
            .as_ref()
            .unwrap()
            .liveness
            .as_ref()
            .unwrap()
            .http
            .as_ref()
            .unwrap();
        assert_eq!(liveness.path, "/health");
        assert_eq!(liveness.port, 8080);
    }
}
