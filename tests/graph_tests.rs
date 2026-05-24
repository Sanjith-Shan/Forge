//! Tests for the dependency graph: topological sort, layering, cycle
//! detection, and the builder's implicit/explicit edge inference.

use forge::config::{parse_str, validate};
use forge::graph::{self, DepGraph};

/// Build a dependency graph straight from a TOML fixture.
fn graph_from(toml: &str) -> Result<DepGraph, Vec<String>> {
    let raw = parse_str(toml).expect("valid TOML");
    let board = validate(raw).expect("valid config").board;
    graph::build(&board).map_err(|errs| errs.into_iter().map(|e| e.message).collect())
}

#[test]
fn linear_chain_sorts_dependencies_first() {
    // a depends on b depends on c  =>  c, b, a.
    let mut g = DepGraph::new();
    g.add_edge("a", "b");
    g.add_edge("b", "c");
    assert_eq!(g.topological_sort().unwrap(), vec!["c", "b", "a"]);
}

#[test]
fn diamond_layers_correctly() {
    // a depends on b and c; both depend on d.
    let mut g = DepGraph::new();
    g.add_edge("a", "b");
    g.add_edge("a", "c");
    g.add_edge("b", "d");
    g.add_edge("c", "d");

    let layers = g.layers().unwrap();
    assert_eq!(layers, vec![vec!["d"], vec!["b", "c"], vec!["a"]]);
    assert_eq!(g.depth_of("d"), 0);
    assert_eq!(g.depth_of("a"), 2);

    // d must come first and a last in any valid linearization.
    let order = g.topological_sort().unwrap();
    assert_eq!(order.first().unwrap(), "d");
    assert_eq!(order.last().unwrap(), "a");
}

#[test]
fn cycle_is_detected_with_path() {
    // a depends on b, b depends on a.
    let mut g = DepGraph::new();
    g.add_edge("a", "b");
    g.add_edge("b", "a");

    let cycle = g.detect_cycle().expect_err("should find a cycle");
    // Path starts and ends at the same node and visits both members.
    assert_eq!(cycle.first(), cycle.last());
    assert!(cycle.contains(&"a".to_string()));
    assert!(cycle.contains(&"b".to_string()));
    assert!(g.topological_sort().is_err());
}

#[test]
fn independent_nodes_sort_alphabetically() {
    let mut g = DepGraph::new();
    for label in ["echo", "alpha", "delta", "bravo", "charlie"] {
        g.add_node(label);
    }
    assert_eq!(
        g.topological_sort().unwrap(),
        vec!["alpha", "bravo", "charlie", "delta", "echo"]
    );
    // All independent => a single layer.
    assert_eq!(g.layers().unwrap().len(), 1);
}

#[test]
fn power_pin_creates_dependency_on_gpio() {
    let toml = r#"
        [board]
        name = "p"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 12
        mode = "output"
        label = "sensor_power"
        [[i2c]]
        bus = 0
        sda_pin = 1
        scl_pin = 2
        speed_khz = 100
        label = "bus0"
        [[i2c.device]]
        address = 0x10
        label = "imu"
        power_pin = 12
    "#;
    let g = graph_from(toml).expect("valid graph");
    let deps: Vec<String> = g.dependencies("imu").cloned().collect();
    assert!(deps.contains(&"sensor_power".to_string()), "deps: {deps:?}");
    assert!(deps.contains(&"bus0".to_string()));
}

#[test]
fn spi_cs_pin_creates_implicit_dependency() {
    let toml = r#"
        [board]
        name = "s"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 5
        mode = "output"
        label = "cs_flash"
        [[spi]]
        bus = 0
        mosi_pin = 23
        miso_pin = 19
        sck_pin = 18
        label = "bus0"
        [[spi.device]]
        cs_pin = 5
        label = "flash"
        mode = 0
        speed_mhz = 1
    "#;
    let g = graph_from(toml).expect("valid graph");
    let deps: Vec<String> = g.dependencies("flash").cloned().collect();
    assert!(deps.contains(&"cs_flash".to_string()), "deps: {deps:?}");
}

#[test]
fn missing_depends_on_target_is_an_error() {
    let toml = r#"
        [board]
        name = "m"
        mcu = "x"
        clock_mhz = 8
        [[i2c]]
        bus = 0
        sda_pin = 1
        scl_pin = 2
        speed_khz = 100
        label = "bus0"
        [[i2c.device]]
        address = 0x10
        label = "thing"
        depends_on = "ghost"
    "#;
    let errors = graph_from(toml).expect_err("should fail");
    assert!(
        errors.iter().any(|e| e.contains("ghost")),
        "errors: {errors:?}"
    );
}

#[test]
fn cycle_via_depends_on_is_reported() {
    let errors = graph_from(include_str!("fixtures/invalid_cycle.toml")).expect_err("cyclic");
    assert!(
        errors.iter().any(|e| e.contains("Circular dependency")),
        "errors: {errors:?}"
    );
}

#[test]
fn valid_dependencies_fixture_builds() {
    let g = graph_from(include_str!("fixtures/valid_dependencies.toml")).expect("valid");
    // imu depends (transitively) on the mux; check the direct edge from imu.
    let deps: Vec<String> = g.dependencies("imu").cloned().collect();
    assert!(deps.contains(&"i2c_mux".to_string()), "deps: {deps:?}");
}
