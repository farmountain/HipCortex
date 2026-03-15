// Capability Registry - Track system capabilities and limitations
//
// Maintains a registry of all operations the system can perform,
// with their resource requirements and current limitations.

use std::collections::HashMap;

/// Describes a capability that the system can perform
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityDescriptor {
    /// Unique name of the capability (e.g., "temporal_insert", "symbolic_query")
    pub name: String,
    
    /// Human-readable description of what this capability does
    pub description: String,
    
    /// Required CPU percentage (0-100)
    pub required_cpu_percent: f64,
    
    /// Required memory in megabytes
    pub required_memory_mb: f64,
    
    /// Current limitations affecting this capability
    pub limitations: Vec<Limitation>,
}

/// Represents a limitation on a capability
#[derive(Debug, Clone, PartialEq)]
pub enum Limitation {
    /// Capability is temporarily disabled
    Disabled { reason: String },
    
    /// Capability has degraded performance
    Degraded { 
        reason: String,
        expected_slowdown_factor: f64, // e.g., 2.0 means 2x slower
    },
    
    /// Capability requires additional resources
    ResourceConstrained {
        resource_type: String, // "cpu", "memory", "disk", "network"
        shortage_percent: f64, // e.g., 20.0 means 20% short of ideal
    },
    
    /// Capability has increased error rate
    Unreliable {
        current_error_rate: f64, // e.g., 0.05 means 5% errors
        baseline_error_rate: f64,
    },
}

impl Limitation {
    /// Check if this is a critical limitation that should block execution
    pub fn is_critical(&self) -> bool {
        match self {
            Limitation::Disabled { .. } => true,
            Limitation::Degraded { expected_slowdown_factor, .. } if *expected_slowdown_factor > 10.0 => true,
            Limitation::ResourceConstrained { shortage_percent, .. } if *shortage_percent > 50.0 => true,
            Limitation::Unreliable { current_error_rate, .. } if *current_error_rate > 0.5 => true,
            _ => false,
        }
    }

    /// Get human-readable description of the limitation
    pub fn describe(&self) -> String {
        match self {
            Limitation::Disabled { reason } => format!("DISABLED: {}", reason),
            Limitation::Degraded { reason, expected_slowdown_factor } => {
                format!("DEGRADED: {} ({}x slower)", reason, expected_slowdown_factor)
            }
            Limitation::ResourceConstrained { resource_type, shortage_percent } => {
                format!("RESOURCE CONSTRAINED: {} shortage of {:.1}%", resource_type, shortage_percent)
            }
            Limitation::Unreliable { current_error_rate, baseline_error_rate } => {
                format!(
                    "UNRELIABLE: {:.1}% error rate (baseline: {:.1}%)",
                    current_error_rate * 100.0,
                    baseline_error_rate * 100.0
                )
            }
        }
    }
}

/// Registry of all system capabilities
pub struct CapabilityRegistry {
    capabilities: HashMap<String, CapabilityDescriptor>,
}

impl CapabilityRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    /// Register a new capability
    ///
    /// # Errors
    /// Returns error if capability with same name already registered
    pub fn register(&mut self, descriptor: CapabilityDescriptor) -> Result<(), String> {
        if self.capabilities.contains_key(&descriptor.name) {
            return Err(format!("Capability '{}' already registered", descriptor.name));
        }

        self.capabilities.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Update an existing capability (replace)
    ///
    /// # Errors
    /// Returns error if capability not found
    pub fn update(&mut self, descriptor: CapabilityDescriptor) -> Result<(), String> {
        if !self.capabilities.contains_key(&descriptor.name) {
            return Err(format!("Capability '{}' not found", descriptor.name));
        }

        self.capabilities.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Get capability descriptor
    ///
    /// # Errors
    /// Returns error if capability not found
    pub fn get(&self, name: &str) -> Result<CapabilityDescriptor, String> {
        self.capabilities
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Capability '{}' not found", name))
    }

    /// Check if capability is registered
    pub fn has(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    /// Add a limitation to a capability
    ///
    /// # Errors
    /// Returns error if capability not found
    pub fn add_limitation(&mut self, capability_name: &str, limitation: Limitation) -> Result<(), String> {
        let descriptor = self.capabilities
            .get_mut(capability_name)
            .ok_or_else(|| format!("Capability '{}' not found", capability_name))?;

        descriptor.limitations.push(limitation);
        Ok(())
    }

    /// Remove a limitation from a capability
    ///
    /// Removes the first limitation that matches the discriminant (type)
    ///
    /// # Errors
    /// Returns error if capability not found
    pub fn remove_limitation(&mut self, capability_name: &str, limitation_type: &str) -> Result<bool, String> {
        let descriptor = self.capabilities
            .get_mut(capability_name)
            .ok_or_else(|| format!("Capability '{}' not found", capability_name))?;

        let initial_len = descriptor.limitations.len();
        descriptor.limitations.retain(|lim| {
            let lim_type = match lim {
                Limitation::Disabled { .. } => "disabled",
                Limitation::Degraded { .. } => "degraded",
                Limitation::ResourceConstrained { .. } => "resource_constrained",
                Limitation::Unreliable { .. } => "unreliable",
            };
            lim_type != limitation_type
        });

        Ok(descriptor.limitations.len() < initial_len)
    }

    /// Get all registered capability names
    pub fn list_capabilities(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    /// Check if capability has any critical limitations
    pub fn has_critical_limitations(&self, name: &str) -> Result<bool, String> {
        let descriptor = self.get(name)?;
        Ok(descriptor.limitations.iter().any(|lim| lim.is_critical()))
    }

    /// Get count of registered capabilities
    pub fn count(&self) -> usize {
        self.capabilities.len()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_descriptor(name: &str) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: name.to_string(),
            description: format!("Test capability: {}", name),
            required_cpu_percent: 20.0,
            required_memory_mb: 512.0,
            limitations: vec![],
        }
    }

    #[test]
    fn test_register_capability() {
        let mut registry = CapabilityRegistry::new();
        let desc = create_test_descriptor("test_op");

        assert!(registry.register(desc).is_ok());
        assert_eq!(registry.count(), 1);
        assert!(registry.has("test_op"));
    }

    #[test]
    fn test_register_duplicate_fails() {
        let mut registry = CapabilityRegistry::new();
        let desc1 = create_test_descriptor("test_op");
        let desc2 = create_test_descriptor("test_op");

        assert!(registry.register(desc1).is_ok());
        assert!(registry.register(desc2).is_err());
    }

    #[test]
    fn test_get_capability() {
        let mut registry = CapabilityRegistry::new();
        let desc = create_test_descriptor("test_op");

        registry.register(desc.clone()).unwrap();
        
        let retrieved = registry.get("test_op");
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().name, "test_op");
    }

    #[test]
    fn test_get_nonexistent_capability() {
        let registry = CapabilityRegistry::new();
        assert!(registry.get("nonexistent").is_err());
    }

    #[test]
    fn test_update_capability() {
        let mut registry = CapabilityRegistry::new();
        let mut desc = create_test_descriptor("test_op");

        registry.register(desc.clone()).unwrap();

        desc.required_cpu_percent = 50.0;
        assert!(registry.update(desc).is_ok());

        let updated = registry.get("test_op").unwrap();
        assert_eq!(updated.required_cpu_percent, 50.0);
    }

    #[test]
    fn test_update_nonexistent_fails() {
        let mut registry = CapabilityRegistry::new();
        let desc = create_test_descriptor("test_op");

        assert!(registry.update(desc).is_err());
    }

    #[test]
    fn test_add_limitation() {
        let mut registry = CapabilityRegistry::new();
        let desc = create_test_descriptor("test_op");

        registry.register(desc).unwrap();

        let limitation = Limitation::Degraded {
            reason: "High load".to_string(),
            expected_slowdown_factor: 2.0,
        };

        assert!(registry.add_limitation("test_op", limitation).is_ok());

        let updated = registry.get("test_op").unwrap();
        assert_eq!(updated.limitations.len(), 1);
    }

    #[test]
    fn test_remove_limitation() {
        let mut registry = CapabilityRegistry::new();
        let mut desc = create_test_descriptor("test_op");
        
        desc.limitations.push(Limitation::Degraded {
            reason: "Test".to_string(),
            expected_slowdown_factor: 2.0,
        });

        registry.register(desc).unwrap();

        let removed = registry.remove_limitation("test_op", "degraded");
        assert!(removed.is_ok());
        assert!(removed.unwrap());

        let updated = registry.get("test_op").unwrap();
        assert_eq!(updated.limitations.len(), 0);
    }

    #[test]
    fn test_list_capabilities() {
        let mut registry = CapabilityRegistry::new();
        
        registry.register(create_test_descriptor("op1")).unwrap();
        registry.register(create_test_descriptor("op2")).unwrap();
        registry.register(create_test_descriptor("op3")).unwrap();

        let caps = registry.list_capabilities();
        assert_eq!(caps.len(), 3);
        assert!(caps.contains(&"op1".to_string()));
        assert!(caps.contains(&"op2".to_string()));
        assert!(caps.contains(&"op3".to_string()));
    }

    #[test]
    fn test_limitation_is_critical() {
        let disabled = Limitation::Disabled { reason: "Maintenance".to_string() };
        assert!(disabled.is_critical());

        let severe_degraded = Limitation::Degraded {
            reason: "Overload".to_string(),
            expected_slowdown_factor: 15.0,
        };
        assert!(severe_degraded.is_critical());

        let mild_degraded = Limitation::Degraded {
            reason: "Load".to_string(),
            expected_slowdown_factor: 2.0,
        };
        assert!(!mild_degraded.is_critical());

        let severe_resource = Limitation::ResourceConstrained {
            resource_type: "memory".to_string(),
            shortage_percent: 60.0,
        };
        assert!(severe_resource.is_critical());

        let high_error = Limitation::Unreliable {
            current_error_rate: 0.6,
            baseline_error_rate: 0.01,
        };
        assert!(high_error.is_critical());
    }

    #[test]
    fn test_has_critical_limitations() {
        let mut registry = CapabilityRegistry::new();
        let mut desc = create_test_descriptor("test_op");

        desc.limitations.push(Limitation::Degraded {
            reason: "Minor load".to_string(),
            expected_slowdown_factor: 1.5,
        });

        registry.register(desc).unwrap();
        assert!(!registry.has_critical_limitations("test_op").unwrap());

        registry.add_limitation("test_op", Limitation::Disabled {
            reason: "Emergency maintenance".to_string(),
        }).unwrap();

        assert!(registry.has_critical_limitations("test_op").unwrap());
    }

    #[test]
    fn test_limitation_describe() {
        let disabled = Limitation::Disabled { reason: "Maintenance".to_string() };
        assert!(disabled.describe().contains("DISABLED"));
        assert!(disabled.describe().contains("Maintenance"));

        let degraded = Limitation::Degraded {
            reason: "High load".to_string(),
            expected_slowdown_factor: 3.5,
        };
        assert!(degraded.describe().contains("DEGRADED"));
        assert!(degraded.describe().contains("3.5x slower"));
    }
}
