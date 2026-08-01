//! Prints the DDL this dialect emits, so a reader can see it without
//! reading the generator.

fn main() {
    print!(
        "{}",
        openehr_store::ddl_script(&openehr_mssql::MssqlDialect)
    );
}
