/// Test naga-based WGSL validation
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_naga_validation_with_valid_wgsl() {
    // Test that valid WGSL passes naga validation
    let valid_shader = r#"
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#;

    // Direct naga validation test
    let result = naga::front::wgsl::parse_str(valid_shader);
    assert!(
        result.is_ok(),
        "Valid WGSL should pass naga validation: {:?}",
        result.err()
    );
}

#[test]
fn test_naga_validation_with_syntax_error() {
    // Test that syntax error is caught by naga
    let invalid_shader = r#"
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
// Missing closing brace

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#;

    // Direct naga validation test
    let result = naga::front::wgsl::parse_str(invalid_shader);
    assert!(result.is_err(), "Invalid WGSL should fail naga validation");

    if let Err(e) = result {
        let error_msg = format!("{}", e);
        println!("Caught naga error: {}", error_msg);
        // Should contain information about the syntax error
        assert!(error_msg.len() > 0, "Error message should provide details");
    }
}

#[test]
fn test_naga_validation_with_undefined_variable() {
    // Test that undefined variable is caught by naga
    let invalid_shader = r#"
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(undefined_variable, 0.0, 0.0, 1.0);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#;

    let result = naga::front::wgsl::parse_str(invalid_shader);
    assert!(
        result.is_err(),
        "WGSL with undefined variable should fail naga validation"
    );
}

#[test]
fn test_naga_validation_with_type_mismatch() {
    // Test that type mismatch is caught by naga
    let invalid_shader = r#"
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let x: f32 = vec3<f32>(1.0, 0.0, 0.0); // Type mismatch
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#;

    let result = naga::front::wgsl::parse_str(invalid_shader);
    assert!(
        result.is_err(),
        "WGSL with type mismatch should fail naga validation"
    );
}

#[test]
fn test_hot_reload_integration_with_naga() {
    // Test that hot reload configuration works with naga validation
    let mut temp_shader = NamedTempFile::new().expect("Failed to create temp file");

    // Write valid shader
    writeln!(
        temp_shader,
        r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}}

struct VertexOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}}
"#
    )
    .expect("Failed to write shader");

    let reloader = shekere::hot_reload::HotReloader::new(temp_shader.path());
    assert!(
        reloader.is_ok(),
        "HotReloader should be created with valid shader"
    );
}

#[test]
fn test_naga_catches_bevy_specific_errors() {
    // Test that naga catches errors that would cause black screen in Bevy
    let problematic_shader = r#"
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let invalid_call = some_undefined_function();
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
"#;

    let result = naga::front::wgsl::parse_str(problematic_shader);
    assert!(
        result.is_err(),
        "Shader with undefined function should fail validation"
    );
}
