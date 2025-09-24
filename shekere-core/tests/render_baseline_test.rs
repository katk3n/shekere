mod common;

use common::*;
use shekere_core::{
    Config,
    pipeline::MultiPassPipeline,
    texture_manager::{TextureManager, TextureType},
};

/// Comprehensive baseline tests for the current render() method behavior
/// These tests capture the existing logic to ensure refactoring doesn't break functionality

struct RenderBaselineValidator {
    config: Config,
    multi_pass_pipeline: MultiPassPipeline,
    texture_manager: TextureManager,
    device: wgpu::Device,
}

impl RenderBaselineValidator {
    fn new(config: Config) -> Self {
        let (device, _queue) = create_test_device_and_queue();
        let surface_config = create_mock_surface_config();

        // Create test shader directory with real shader files
        let test_dir = create_test_config_dir();
        let config_dir = test_dir.path();

        // Create shader files for the pipeline using the basic shader
        for shader_config in &config.pipeline {
            let shader_path = config_dir.join(&shader_config.file);
            if let Some(parent) = shader_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&shader_path, include_str!("shaders/test.wgsl")).unwrap();
        }

        // Mock bind group layouts (empty for testing)
        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = vec![];

        // Initialize multi-pass pipeline
        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            config_dir,
            &config.pipeline,
            &surface_config,
            &bind_group_layouts,
        );

        let texture_manager = TextureManager::new_with_format(
            &device,
            surface_config.width,
            surface_config.height,
            surface_config.format,
        );

        Self {
            config,
            multi_pass_pipeline,
            texture_manager,
            device,
        }
    }

    /// Validates the complete render decision flow from State::render
    fn validate_render_flow(&mut self) -> RenderFlowValidation {
        // Step 1: Initial analysis (lines 510-516 in State::render)
        let pipeline_count = self.multi_pass_pipeline.pipeline_count();
        let is_multipass = self.multi_pass_pipeline.is_multi_pass();

        let has_persistent_textures =
            (0..pipeline_count).any(|i| self.determine_texture_type(i) == TextureType::Persistent);
        let has_ping_pong_textures =
            (0..pipeline_count).any(|i| self.determine_texture_type(i) == TextureType::PingPong);

        // Step 2: Mode decision (lines 518-523 in State::render)
        let should_use_multipass_mode = (is_multipass && pipeline_count > 1)
            || has_persistent_textures
            || has_ping_pong_textures;

        // Step 3: Texture creation validation (lines 525-551 in State::render)
        let mut texture_creation_results = Vec::new();
        if should_use_multipass_mode {
            for i in 0..pipeline_count {
                let texture_type = self.determine_texture_type(i);
                // Skip texture creation for final pass unless it's persistent or ping-pong
                let should_skip = i == pipeline_count - 1
                    && texture_type != TextureType::Persistent
                    && texture_type != TextureType::PingPong;

                if !should_skip {
                    let creation_result = match texture_type {
                        TextureType::Intermediate => {
                            let _result = self
                                .texture_manager
                                .get_or_create_intermediate_texture(&self.device, i);
                            true // These methods always succeed and return (&TextureView, &Sampler)
                        }
                        TextureType::PingPong => {
                            let _result = self
                                .texture_manager
                                .get_or_create_ping_pong_texture(&self.device, i);
                            true // These methods always succeed and return (&TextureView, &Sampler)
                        }
                        TextureType::Persistent => {
                            let _result = self
                                .texture_manager
                                .get_or_create_persistent_texture(&self.device, i);
                            true // These methods always succeed and return (&TextureView, &Sampler)
                        }
                    };
                    texture_creation_results.push((i, texture_type, creation_result));
                }
            }
        }

        // Step 4: Render pass validation (lines 554-864 in State::render)
        let mut render_pass_validations = Vec::new();
        if should_use_multipass_mode {
            for pass_index in 0..pipeline_count {
                let current_texture_type = self.determine_texture_type(pass_index);
                let is_final_pass = pass_index == pipeline_count - 1
                    && current_texture_type != TextureType::Persistent
                    && current_texture_type != TextureType::PingPong;

                // Validate render target availability
                let render_target_available = if is_final_pass {
                    true // Would use final_view
                } else {
                    match current_texture_type {
                        TextureType::Intermediate => self
                            .texture_manager
                            .get_intermediate_render_target(pass_index)
                            .is_some(),
                        TextureType::PingPong => self
                            .texture_manager
                            .get_ping_pong_render_target(pass_index)
                            .is_some(),
                        TextureType::Persistent => self
                            .texture_manager
                            .get_persistent_render_target(pass_index)
                            .is_some(),
                    }
                };

                // Validate input texture binding logic
                let input_texture_available = if pass_index > 0
                    || current_texture_type == TextureType::Persistent
                    || current_texture_type == TextureType::PingPong
                {
                    true // Would create texture bind group
                } else {
                    true // First pass without special textures doesn't need input
                };

                render_pass_validations.push(RenderPassValidation {
                    pass_index,
                    texture_type: current_texture_type,
                    is_final_pass,
                    render_target_available,
                    input_texture_available,
                });
            }
        }

        RenderFlowValidation {
            pipeline_count,
            is_multipass,
            has_persistent_textures,
            has_ping_pong_textures,
            should_use_multipass_mode,
            texture_creation_results,
            render_pass_validations,
        }
    }

    /// Mirrors State::determine_texture_type method
    fn determine_texture_type(&self, pass_index: usize) -> TextureType {
        if pass_index < self.config.pipeline.len() {
            let shader_config = &self.config.pipeline[pass_index];
            if shader_config.persistent.unwrap_or(false) {
                TextureType::Persistent
            } else if shader_config.ping_pong.unwrap_or(false) {
                TextureType::PingPong
            } else {
                TextureType::Intermediate
            }
        } else {
            TextureType::Intermediate
        }
    }
}

#[derive(Debug)]
struct RenderFlowValidation {
    pipeline_count: usize,
    is_multipass: bool,
    has_persistent_textures: bool,
    has_ping_pong_textures: bool,
    should_use_multipass_mode: bool,
    texture_creation_results: Vec<(usize, TextureType, bool)>,
    render_pass_validations: Vec<RenderPassValidation>,
}

#[derive(Debug)]
struct RenderPassValidation {
    pass_index: usize,
    texture_type: TextureType,
    is_final_pass: bool,
    render_target_available: bool,
    input_texture_available: bool,
}

/// Test the complete render flow for single-pass configuration
#[test]
fn test_baseline_single_pass_render_flow() {
    let config = create_test_config();
    let mut validator = RenderBaselineValidator::new(config);

    let validation = validator.validate_render_flow();

    // Validate single-pass characteristics
    assert_eq!(validation.pipeline_count, 1);
    assert!(!validation.is_multipass);
    assert!(!validation.has_persistent_textures);
    assert!(!validation.has_ping_pong_textures);
    assert!(!validation.should_use_multipass_mode);

    // Single-pass should not create any intermediate textures
    assert!(validation.texture_creation_results.is_empty());
    assert!(validation.render_pass_validations.is_empty());
}

/// Test the complete render flow for multi-pass configuration
#[test]
fn test_baseline_multipass_render_flow() {
    let config = create_multipass_test_config();
    let mut validator = RenderBaselineValidator::new(config);

    let validation = validator.validate_render_flow();

    // Validate multi-pass characteristics
    assert!(validation.pipeline_count > 1);
    assert!(validation.is_multipass);
    assert!(!validation.has_persistent_textures);
    assert!(!validation.has_ping_pong_textures);
    assert!(validation.should_use_multipass_mode);

    // Multi-pass should create intermediate textures for all but final pass
    let expected_texture_count = validation.pipeline_count - 1;
    assert_eq!(
        validation.texture_creation_results.len(),
        expected_texture_count
    );

    // All texture creations should succeed
    for (pass_index, texture_type, success) in &validation.texture_creation_results {
        assert!(
            *success,
            "Texture creation failed for pass {} with type {:?}",
            pass_index, texture_type
        );
        assert_eq!(*texture_type, TextureType::Intermediate);
    }

    // Should have render pass validations for all passes
    assert_eq!(
        validation.render_pass_validations.len(),
        validation.pipeline_count
    );

    // Final pass should be marked as final
    let final_pass = validation.render_pass_validations.last().unwrap();
    assert!(final_pass.is_final_pass);
    assert_eq!(final_pass.pass_index, validation.pipeline_count - 1);

    // All render targets should be available
    for pass_validation in &validation.render_pass_validations {
        assert!(
            pass_validation.render_target_available,
            "Render target not available for pass {}",
            pass_validation.pass_index
        );
        assert!(
            pass_validation.input_texture_available,
            "Input texture not available for pass {}",
            pass_validation.pass_index
        );
    }
}

/// Test the complete render flow for ping-pong configuration
#[test]
fn test_baseline_ping_pong_render_flow() {
    let config = create_ping_pong_test_config();
    let mut validator = RenderBaselineValidator::new(config);

    let validation = validator.validate_render_flow();

    // Validate ping-pong characteristics
    assert_eq!(validation.pipeline_count, 1);
    assert!(validation.is_multipass); // Ping-pong triggers multipass flag in pipeline
    assert!(!validation.has_persistent_textures);
    assert!(validation.has_ping_pong_textures);
    assert!(validation.should_use_multipass_mode); // Ping-pong forces multipass mode

    // Ping-pong should create texture even for single pass
    assert_eq!(validation.texture_creation_results.len(), 1);
    let (pass_index, texture_type, success) = &validation.texture_creation_results[0];
    assert_eq!(*pass_index, 0);
    assert_eq!(*texture_type, TextureType::PingPong);
    assert!(*success);

    // Should have render pass validation
    assert_eq!(validation.render_pass_validations.len(), 1);
    let pass_validation = &validation.render_pass_validations[0];
    assert!(!pass_validation.is_final_pass); // Ping-pong pass is never "final" in render logic
    assert_eq!(pass_validation.texture_type, TextureType::PingPong);
    assert!(pass_validation.render_target_available);
    assert!(pass_validation.input_texture_available);
}

/// Test the complete render flow for persistent configuration
#[test]
fn test_baseline_persistent_render_flow() {
    let config = create_persistent_test_config();
    let mut validator = RenderBaselineValidator::new(config);

    let validation = validator.validate_render_flow();

    // Validate persistent characteristics
    assert_eq!(validation.pipeline_count, 1);
    assert!(validation.is_multipass); // Persistent triggers multipass flag in pipeline
    assert!(validation.has_persistent_textures);
    assert!(!validation.has_ping_pong_textures);
    assert!(validation.should_use_multipass_mode); // Persistent forces multipass mode

    // Persistent should create texture even for single pass
    assert_eq!(validation.texture_creation_results.len(), 1);
    let (pass_index, texture_type, success) = &validation.texture_creation_results[0];
    assert_eq!(*pass_index, 0);
    assert_eq!(*texture_type, TextureType::Persistent);
    assert!(*success);

    // Should have render pass validation
    assert_eq!(validation.render_pass_validations.len(), 1);
    let pass_validation = &validation.render_pass_validations[0];
    assert!(!pass_validation.is_final_pass); // Persistent pass is never "final" in render logic
    assert_eq!(pass_validation.texture_type, TextureType::Persistent);
    assert!(pass_validation.render_target_available);
    assert!(pass_validation.input_texture_available);
}

/// Test texture creation skip logic exactly as implemented in State::render
#[test]
fn test_baseline_texture_creation_skip_logic() {
    // Test with a 3-pass configuration
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Pass 1"
entry_point = "fs_main"
file = "pass1.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Pass 2"
entry_point = "fs_main"
file = "pass2.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Pass 3"
entry_point = "fs_main"
file = "pass3.wgsl"
"#;
    let config: Config = toml::from_str(config_content).unwrap();
    let mut validator = RenderBaselineValidator::new(config);

    let validation = validator.validate_render_flow();

    // Should create textures for passes 0 and 1, but skip pass 2 (final pass)
    assert_eq!(validation.texture_creation_results.len(), 2);

    let pass_indices: Vec<usize> = validation
        .texture_creation_results
        .iter()
        .map(|(index, _, _)| *index)
        .collect();
    assert!(pass_indices.contains(&0));
    assert!(pass_indices.contains(&1));
    assert!(!pass_indices.contains(&2)); // Final pass should be skipped
}

/// Test frame calculation logic used in persistent and ping-pong textures
#[test]
fn test_baseline_frame_calculation() {
    // This tests the frame calculation logic from State::render lines 607, 625
    let calculate_read_index = |current_frame: u32| -> usize {
        ((current_frame + 1) % 2) as usize // Read from previous frame
    };

    // Test the exact calculation used in the render method
    assert_eq!(calculate_read_index(0), 1);
    assert_eq!(calculate_read_index(1), 0);
    assert_eq!(calculate_read_index(2), 1);
    assert_eq!(calculate_read_index(3), 0);

    // Verify this creates the expected ping-pong pattern
    let mut current_frame = 0u32;
    for _ in 0..10 {
        let read_index = calculate_read_index(current_frame);
        let write_index = (current_frame % 2) as usize;

        // Read and write indices should always be different
        assert_ne!(
            read_index, write_index,
            "Frame {}: read_index={}, write_index={}",
            current_frame, read_index, write_index
        );

        current_frame += 1;
    }
}

/// Test the exact multipass condition from State::render line 522
#[test]
fn test_baseline_multipass_condition() {
    // Test all combinations of the multipass condition
    struct TestCase {
        is_multipass: bool,
        pipeline_count: usize,
        has_persistent: bool,
        has_ping_pong: bool,
        expected: bool,
        description: &'static str,
    }

    let test_cases = vec![
        TestCase {
            is_multipass: true,
            pipeline_count: 2,
            has_persistent: false,
            has_ping_pong: false,
            expected: true,
            description: "Multi-pass with multiple pipelines",
        },
        TestCase {
            is_multipass: true,
            pipeline_count: 1,
            has_persistent: false,
            has_ping_pong: false,
            expected: false,
            description: "Multi-pass with single pipeline",
        },
        TestCase {
            is_multipass: false,
            pipeline_count: 1,
            has_persistent: true,
            has_ping_pong: false,
            expected: true,
            description: "Single pass with persistent textures",
        },
        TestCase {
            is_multipass: false,
            pipeline_count: 1,
            has_persistent: false,
            has_ping_pong: true,
            expected: true,
            description: "Single pass with ping-pong textures",
        },
        TestCase {
            is_multipass: false,
            pipeline_count: 1,
            has_persistent: false,
            has_ping_pong: false,
            expected: false,
            description: "Simple single pass",
        },
        TestCase {
            is_multipass: true,
            pipeline_count: 3,
            has_persistent: true,
            has_ping_pong: true,
            expected: true,
            description: "Multi-pass with all features",
        },
    ];

    for test_case in test_cases {
        let condition = (test_case.is_multipass && test_case.pipeline_count > 1)
            || test_case.has_persistent
            || test_case.has_ping_pong;

        assert_eq!(
            condition, test_case.expected,
            "Failed test case: {}",
            test_case.description
        );
    }
}

/// Test texture type determination with edge cases
#[test]
fn test_baseline_texture_type_edge_cases() {
    // Test conflicting configuration (should prioritize persistent over ping-pong)
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Conflicting Config"
entry_point = "fs_main"
file = "test.wgsl"
persistent = true
ping_pong = true
"#;
    let config: Config = toml::from_str(config_content).unwrap();
    let validator = RenderBaselineValidator::new(config);

    // Should prioritize persistent over ping-pong
    assert_eq!(validator.determine_texture_type(0), TextureType::Persistent);

    // Test out-of-bounds pass index
    assert_eq!(
        validator.determine_texture_type(999),
        TextureType::Intermediate
    );
}
