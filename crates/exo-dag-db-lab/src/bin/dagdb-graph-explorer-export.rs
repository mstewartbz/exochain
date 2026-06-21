fn main() {
    eprintln!("{}", export_binary_message());
}

fn export_binary_message() -> &'static str {
    "graph_explorer_export_binary_requires_approved_operator_runtime"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_explorer_export_binary_message_is_generic() {
        main();
        assert_eq!(
            export_binary_message(),
            "graph_explorer_export_binary_requires_approved_operator_runtime"
        );
        assert!(!export_binary_message().contains("postgres://"));
    }
}
