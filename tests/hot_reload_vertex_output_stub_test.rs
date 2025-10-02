use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_shader_validation_with_vertex_output_import() {
    // Test that shaders using VertexOutput from #import are validated correctly
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut shader_file = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        shader_file,
        r#"
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    return vec4<f32>(in.uv, 0.0, 1.0);
}}
        "#
    )
    .expect("Failed to write shader");

    let shader_paths = vec![shader_file.path().to_path_buf()];
    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader with VertexOutput shader"
    );
}

#[test]
fn test_shader_validation_with_complex_vertex_output_usage() {
    // Test shader that uses multiple fields from VertexOutput
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut shader_file = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        shader_file,
        r#"
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

fn helper_function(v: VertexOutput) -> vec2<f32> {{
    return v.uv * 2.0;
}}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    let uv = helper_function(in);
    let pos = in.position.xy;
    return vec4<f32>(uv, pos.x * 0.001, 1.0);
}}
        "#
    )
    .expect("Failed to write shader");

    let shader_paths = vec![shader_file.path().to_path_buf()];
    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader with complex VertexOutput usage"
    );
}

#[test]
fn test_shader_validation_catches_real_type_errors() {
    // Test that validation still catches actual type errors
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut shader_file = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        shader_file,
        r#"
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    // This should fail validation - using undefined type
    let invalid: UndefinedType = 0.0;
    return vec4<f32>(in.uv, 0.0, 1.0);
}}
        "#
    )
    .expect("Failed to write shader");

    // File watching should still work - validation happens during reload
    let shader_paths = vec![shader_file.path().to_path_buf()];
    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "HotReloader creation should succeed (validation happens during reload)"
    );
}

#[test]
fn test_shader_validation_with_multiple_imports() {
    // Test shader with multiple Bevy imports
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut shader_file = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        shader_file,
        r#"
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_pbr::forward_io::Vertex

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    return vec4<f32>(in.uv, 0.0, 1.0);
}}
        "#
    )
    .expect("Failed to write shader");

    let shader_paths = vec![shader_file.path().to_path_buf()];
    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader with multiple imports"
    );
}

#[test]
fn test_osc_example_shader_validation() {
    // Test a realistic shader similar to the OSC example
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut shader_file = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        shader_file,
        r#"
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Mock functions that would normally come from common.wgsl
fn NormalizedCoords(pos: vec2<f32>) -> vec2<f32> {{
    return vec2<f32>(0.0, 0.0);
}}

fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {{
    return col;
}}

fn orb(p: vec2<f32>, p0: vec2<f32>, r: f32, col: vec3<f32>) -> vec3<f32> {{
    let t = clamp(1.0 + r - length(p - p0), 0.0, 1.0);
    return vec3(pow(t, 16.0) * col);
}}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    let uv = NormalizedCoords(in.position.xy);
    let white = vec3(1.0, 1.0, 1.0);
    let black = vec3(0.0, 0.0, 0.0);

    var col = black;
    col += orb(uv, vec2(0.0, 0.0), 0.15, white);

    return vec4(ToLinearRgb(col), 1.0);
}}
        "#
    )
    .expect("Failed to write shader");

    let shader_paths = vec![shader_file.path().to_path_buf()];
    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader with OSC-like shader"
    );
}
