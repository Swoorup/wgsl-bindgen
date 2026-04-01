// The bevy integration test used naga-oil's `#import` syntax which is no longer
// supported since wgsl_bindgen now uses WESL as the sole shader compilation backend.
// Bevy shaders would need to be migrated to WESL's `import package::module;` syntax
// to work with this tool.
