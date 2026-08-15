use crate::procedural_cache::{FSMBackend, FSMState, FSMTransition, ProceduralCache};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillTemplateStep {
    pub state_name: String,
    pub action_pattern: String,
    pub extracted_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTemplate {
    pub id: Uuid,
    pub name: String,
    pub steps: Vec<SkillTemplateStep>,
    pub success_rate: f64,
    pub observation_count: usize,
}

pub struct SkillCompiler;

impl SkillCompiler {
    /// Extract repeating action traces above `success_threshold` and abstract concrete target entities into `$arg` variables.
    pub fn parameterize_trace(
        name: String,
        steps: &[(String, String)],
        success_rate: f64,
        observation_count: usize,
        success_threshold: f64,
        target_entities: &[String],
    ) -> Option<SkillTemplate> {
        if success_rate < success_threshold {
            return None;
        }

        let mut parameterized_steps = Vec::new();
        for (state_name, concrete_action) in steps {
            let mut action_pattern = concrete_action.clone();
            let mut extracted_args = Vec::new();

            for (i, entity) in target_entities.iter().enumerate() {
                if action_pattern.contains(entity) {
                    let arg_var = format!("$arg{}", i);
                    action_pattern = action_pattern.replace(entity, &arg_var);
                    if !extracted_args.contains(&arg_var) {
                        extracted_args.push(arg_var);
                    }
                }
            }

            parameterized_steps.push(SkillTemplateStep {
                state_name: state_name.clone(),
                action_pattern,
                extracted_args,
            });
        }

        Some(SkillTemplate {
            id: Uuid::new_v4(),
            name,
            steps: parameterized_steps,
            success_rate,
            observation_count,
        })
    }

    /// Compile parameterized `SkillTemplate` actions into `FSMTransition` rules inside `ProceduralCache`.
    pub fn compile_and_register_skill<B: FSMBackend>(
        cache: &mut ProceduralCache<B>,
        template: &SkillTemplate,
    ) {
        if template.steps.is_empty() {
            return;
        }

        let first_transition = FSMTransition {
            from: FSMState::Start,
            to: FSMState::Custom(template.steps[0].state_name.clone()),
            condition: Some(template.name.clone()),
        };
        cache.add_transition(first_transition);

        for i in 0..template.steps.len() {
            let current_step = &template.steps[i];
            let from_state = FSMState::Custom(current_step.state_name.clone());
            let to_state = if i + 1 < template.steps.len() {
                FSMState::Custom(template.steps[i + 1].state_name.clone())
            } else {
                FSMState::End
            };

            let transition = FSMTransition {
                from: from_state,
                to: to_state,
                condition: Some(current_step.action_pattern.clone()),
            };
            cache.add_transition(transition);
        }
    }
}
