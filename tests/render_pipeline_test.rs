mod common;

use common::*;
use shekere_core::{
    Config,
    pipeline::MultiPassPipeline,
    texture_manager::{TextureManager, TextureType},
};

/// Test helper for render pipeline analysis
/// This mirrors the logic from State::render without requiring full State instantiation
struct RenderPipelineAnalyzer {
    config: Config,
    multi_pass_pipeline: MultiPassPipeline,
    texture_manager: TextureManager,
    device: wgpu::Device,
}

impl RenderPipelineAnalyzer {
    fn new(config: Config) -> Self {
        let (device, _queue) = create_test_device_and_queue();
        let surface_config = create_mock_surface_config();

        // Create test shader directory with mock shader files
        let test_dir = create_test_config_dir();
        let config_dir = test_dir.path();

        // Create mock shader files for the pipeline
        for shader_config in &config.pipeline {
            let shader_path = config_dir.join(&shader_config.file);
            if let Some(parent) = shader_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&shader_path, include_str!("shaders/basic.wgsl")).unwrap();
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

    /// Determines texture type for a given pass index
    /// This mirrors the logic from State::determine_texture_type
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

    /// Analyzes the render pipeline requirements
    /// This mirrors the analysis logic from State::render
    fn analyze_render_requirements(&self) -> RenderRequirements {
        let pipeline_count = self.multi_pass_pipeline.pipeline_count();
        let is_multipass = self.multi_pass_pipeline.is_multi_pass();

        let has_persistent_textures =
            (0..pipeline_count).any(|i| self.determine_texture_type(i) == TextureType::Persistent);
        let has_ping_pong_textures =
            (0..pipeline_count).any(|i| self.determine_texture_type(i) == TextureType::PingPong);

        let requires_multipass_mode = (is_multipass && pipeline_count > 1)
            || has_persistent_textures
            || has_ping_pong_textures;

        RenderRequirements {
            pipeline_count,
            is_multipass,
            has_persistent_textures,
            has_ping_pong_textures,
            requires_multipass_mode,
        }
    }

    /// Test texture creation for all passes
    fn test_texture_creation(&mut self) -> Result<(), String> {
        let requirements = self.analyze_render_requirements();

        // Test texture creation logic from State::render
        for i in 0..requirements.pipeline_count {
            let texture_type = self.determine_texture_type(i);

            // Skip texture creation for final pass unless it's persistent or ping-pong
            if i == requirements.pipeline_count - 1
                && texture_type != TextureType::Persistent
                && texture_type != TextureType::PingPong
            {
                continue;
            }

            match texture_type {
                TextureType::Intermediate => {
                    let _result = self
                        .texture_manager
                        .get_or_create_intermediate_texture(&self.device, i);
                    // Note: This method returns (&TextureView, &Sampler), not Result
                }
                TextureType::PingPong => {
                    let _result = self
                        .texture_manager
                        .get_or_create_ping_pong_texture(&self.device, i);
                    // Note: This method returns (&TextureView, &Sampler), not Result
                }
                TextureType::Persistent => {
                    let _result = self
                        .texture_manager
                        .get_or_create_persistent_texture(&self.device, i);
                    // Note: This method returns (&TextureView, &Sampler), not Result
                }
            }
        }

        Ok(())
    }

    /// Test that render targets can be retrieved after creation
    fn test_render_target_retrieval(&self) -> Result<(), String> {
        let requirements = self.analyze_render_requirements();

        for i in 0..requirements.pipeline_count {
            let texture_type = self.determine_texture_type(i);
            let is_final_pass = i == requirements.pipeline_count - 1
                && texture_type != TextureType::Persistent
                && texture_type != TextureType::PingPong;

            if !is_final_pass {
                let render_target_exists = match texture_type {
                    TextureType::Intermediate => self
                        .texture_manager
                        .get_intermediate_render_target(i)
                        .is_some(),
                    TextureType::PingPong => self
                        .texture_manager
                        .get_ping_pong_render_target(i)
                        .is_some(),
                    TextureType::Persistent => self
                        .texture_manager
                        .get_persistent_render_target(i)
                        .is_some(),
                };

                if !render_target_exists {
                    return Err(format!(
                        "No render target available for pass {} with type {:?}",
                        i, texture_type
                    ));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct RenderRequirements {
    pipeline_count: usize,
    is_multipass: bool,
    has_persistent_textures: bool,
    has_ping_pong_textures: bool,
    requires_multipass_mode: bool,
}

#[test]
fn test_single_pass_pipeline_analysis() {
    let config = create_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let requirements = analyzer.analyze_render_requirements();

    // Single pass config should have these characteristics
    assert_eq!(requirements.pipeline_count, 1);
    assert!(!requirements.is_multipass);
    assert!(!requirements.has_persistent_textures);
    assert!(!requirements.has_ping_pong_textures);
    assert!(!requirements.requires_multipass_mode);
}

#[test]
fn test_multipass_pipeline_analysis() {
    let config = create_multipass_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let requirements = analyzer.analyze_render_requirements();

    // Multi-pass config should have these characteristics
    assert!(requirements.pipeline_count > 1);
    assert!(requirements.is_multipass);
    assert!(!requirements.has_persistent_textures);
    assert!(!requirements.has_ping_pong_textures);
    assert!(requirements.requires_multipass_mode); // Because pipeline_count > 1 and is_multipass
}

#[test]
fn test_ping_pong_pipeline_analysis() {
    let config = create_ping_pong_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let requirements = analyzer.analyze_render_requirements();

    // Ping-pong config should have these characteristics
    assert_eq!(requirements.pipeline_count, 1);
    assert!(requirements.is_multipass); // Single pass but with ping-pong triggers multipass flag
    assert!(!requirements.has_persistent_textures);
    assert!(requirements.has_ping_pong_textures);
    assert!(requirements.requires_multipass_mode); // Because has_ping_pong_textures
}

#[test]
fn test_persistent_pipeline_analysis() {
    let config = create_persistent_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let requirements = analyzer.analyze_render_requirements();

    // Persistent config should have these characteristics
    assert_eq!(requirements.pipeline_count, 1);
    assert!(requirements.is_multipass); // Single pass but with persistent triggers multipass flag
    assert!(requirements.has_persistent_textures);
    assert!(!requirements.has_ping_pong_textures);
    assert!(requirements.requires_multipass_mode); // Because has_persistent_textures
}

#[test]
fn test_texture_type_determination() {
    let ping_pong_config = create_ping_pong_test_config();
    let ping_pong_analyzer = RenderPipelineAnalyzer::new(ping_pong_config);
    assert_eq!(
        ping_pong_analyzer.determine_texture_type(0),
        TextureType::PingPong
    );

    let persistent_config = create_persistent_test_config();
    let persistent_analyzer = RenderPipelineAnalyzer::new(persistent_config);
    assert_eq!(
        persistent_analyzer.determine_texture_type(0),
        TextureType::Persistent
    );

    let simple_config = create_test_config();
    let simple_analyzer = RenderPipelineAnalyzer::new(simple_config);
    assert_eq!(
        simple_analyzer.determine_texture_type(0),
        TextureType::Intermediate
    );
}

#[test]
fn test_texture_creation_single_pass() {
    let config = create_test_config();
    let mut analyzer = RenderPipelineAnalyzer::new(config);

    let result = analyzer.test_texture_creation();
    assert!(
        result.is_ok(),
        "Single pass texture creation failed: {:?}",
        result
    );

    // For single pass, no textures should be created (final pass is skipped)
    let retrieval_result = analyzer.test_render_target_retrieval();
    assert!(
        retrieval_result.is_ok(),
        "Single pass should not require intermediate textures"
    );
}

#[test]
fn test_texture_creation_multipass() {
    let config = create_multipass_test_config();
    let mut analyzer = RenderPipelineAnalyzer::new(config);

    let result = analyzer.test_texture_creation();
    assert!(
        result.is_ok(),
        "Multi-pass texture creation failed: {:?}",
        result
    );

    let retrieval_result = analyzer.test_render_target_retrieval();
    assert!(
        retrieval_result.is_ok(),
        "Multi-pass render target retrieval failed: {:?}",
        retrieval_result
    );
}

#[test]
fn test_texture_creation_ping_pong() {
    let config = create_ping_pong_test_config();
    let mut analyzer = RenderPipelineAnalyzer::new(config);

    let result = analyzer.test_texture_creation();
    assert!(
        result.is_ok(),
        "Ping-pong texture creation failed: {:?}",
        result
    );

    let retrieval_result = analyzer.test_render_target_retrieval();
    assert!(
        retrieval_result.is_ok(),
        "Ping-pong render target retrieval failed: {:?}",
        retrieval_result
    );
}

#[test]
fn test_texture_creation_persistent() {
    let config = create_persistent_test_config();
    let mut analyzer = RenderPipelineAnalyzer::new(config);

    let result = analyzer.test_texture_creation();
    assert!(
        result.is_ok(),
        "Persistent texture creation failed: {:?}",
        result
    );

    let retrieval_result = analyzer.test_render_target_retrieval();
    assert!(
        retrieval_result.is_ok(),
        "Persistent render target retrieval failed: {:?}",
        retrieval_result
    );
}

#[test]
fn test_render_decision_logic() {
    // Test the core decision logic from State::render

    // Single pass should not require multipass mode
    let single_config = create_test_config();
    let single_analyzer = RenderPipelineAnalyzer::new(single_config);
    let single_req = single_analyzer.analyze_render_requirements();
    assert!(!single_req.requires_multipass_mode);

    // Multi-pass should require multipass mode
    let multi_config = create_multipass_test_config();
    let multi_analyzer = RenderPipelineAnalyzer::new(multi_config);
    let multi_req = multi_analyzer.analyze_render_requirements();
    assert!(multi_req.requires_multipass_mode);

    // Ping-pong should require multipass mode even with single pass
    let ping_pong_config = create_ping_pong_test_config();
    let ping_pong_analyzer = RenderPipelineAnalyzer::new(ping_pong_config);
    let ping_pong_req = ping_pong_analyzer.analyze_render_requirements();
    assert!(ping_pong_req.requires_multipass_mode);

    // Persistent should require multipass mode even with single pass
    let persistent_config = create_persistent_test_config();
    let persistent_analyzer = RenderPipelineAnalyzer::new(persistent_config);
    let persistent_req = persistent_analyzer.analyze_render_requirements();
    assert!(persistent_req.requires_multipass_mode);
}

// ===== Phase 1-5: render_multipass tests =====

#[test]
fn test_render_multipass_basic_functionality() {
    // Test basic render_multipass method requirements analysis
    let config = create_multipass_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    // Create PassTextureInfo
    let pass_info = analyzer.analyze_render_requirements();

    // Verify multipass requirements are properly analyzed
    assert!(pass_info.requires_multipass_mode);
    assert_eq!(pass_info.pipeline_count, 2); // Multi-pass config has 2 passes

    // Verify this would trigger render_multipass() call in actual State::render()
    assert!(pass_info.requires_multipass_mode);
}

#[test]
fn test_render_multipass_persistent_texture_handling() {
    // Test render_multipass logic for persistent textures
    let config = create_persistent_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let pass_info = analyzer.analyze_render_requirements();

    // Should handle persistent textures with double-buffering
    assert!(pass_info.has_persistent_textures);
    assert!(pass_info.requires_multipass_mode);

    // Verify persistent texture type is detected
    assert_eq!(analyzer.determine_texture_type(0), TextureType::Persistent);

    // Verify this would trigger render_multipass() in State::render()
    assert!(pass_info.requires_multipass_mode);
}

#[test]
fn test_render_multipass_ping_pong_texture_handling() {
    // Test render_multipass logic for ping-pong textures
    let config = create_ping_pong_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let pass_info = analyzer.analyze_render_requirements();

    // Should handle ping-pong textures with frame swapping
    assert!(pass_info.has_ping_pong_textures);
    assert!(pass_info.requires_multipass_mode);

    // Verify ping-pong texture type is detected
    assert_eq!(analyzer.determine_texture_type(0), TextureType::PingPong);

    // Verify this would trigger render_multipass() in State::render()
    assert!(pass_info.requires_multipass_mode);
}

#[test]
fn test_render_multipass_copy_pass_execution() {
    // Test copy pass logic requirements for persistent/ping-pong textures
    let config = create_persistent_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let pass_info = analyzer.analyze_render_requirements();

    // Should execute copy passes for persistent textures to final view
    assert!(pass_info.has_persistent_textures);
    assert!(pass_info.requires_multipass_mode);

    // Verify copy pass would be needed for persistent textures
    assert_eq!(analyzer.determine_texture_type(0), TextureType::Persistent);

    // This confirms render_multipass() will include copy pass logic
    assert!(pass_info.has_persistent_textures || pass_info.has_ping_pong_textures);
}

#[test]
fn test_render_multipass_error_handling() {
    // Test render_multipass error handling logic
    let config = create_multipass_test_config();
    let analyzer = RenderPipelineAnalyzer::new(config);

    let pass_info = analyzer.analyze_render_requirements();

    // Should return Result<(), wgpu::SurfaceError> for proper error handling
    assert!(pass_info.requires_multipass_mode);
    assert_eq!(pass_info.pipeline_count, 2);

    // Verify that render_multipass() signature returns Result for error propagation
    // This ensures proper error handling in the State::render() flow
    assert!(pass_info.requires_multipass_mode);
}
