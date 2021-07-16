use schema::backend_schema;
use schema::control_schema;
use schema::signal_schema;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/graphql/backend_query_tmpl.rs");
    println!("cargo:rerun-if-changed=src/graphql/control_query_tmpl.rs");
    println!("cargo:rerun-if-changed=src/graphql/signal_query_tmpl.rs");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let backend_gql = Path::new(&out_dir).join("backend_schema.gql");
    let control_gql = Path::new(&out_dir).join("control_schema.gql");
    let signal_gql = Path::new(&out_dir).join("signal_schema.gql");
    let backend_rs = Path::new(&out_dir).join("backend_query.rs");
    let control_rs = Path::new(&out_dir).join("control_query.rs");
    let signal_rs = Path::new(&out_dir).join("signal_query.rs");

    fs::write(&backend_gql, backend_schema()).unwrap();
    fs::write(&control_gql, control_schema()).unwrap();
    fs::write(&signal_gql, signal_schema()).unwrap();

    fs::write(
        &backend_rs,
        include_str!("src/graphql/backend_query_tmpl.rs")
            .replace("$schema_path$", backend_gql.to_str().unwrap()),
    )
    .unwrap();
    fs::write(
        &control_rs,
        include_str!("src/graphql/control_query_tmpl.rs")
            .replace("$schema_path$", control_gql.to_str().unwrap()),
    )
    .unwrap();
    fs::write(
        &signal_rs,
        include_str!("src/graphql/signal_query_tmpl.rs")
            .replace("$schema_path$", signal_gql.to_str().unwrap()),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
