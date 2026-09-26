


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_quote_safety() -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("Step 1: Loading workspace...");
        let ws = workspace();
        eprintln!("Step 1: OK");

        eprintln!("Step 2: Getting first enum...");
        let enums: Vec<&sema::ResolvedEnum> = ws.enums().collect();
        eprintln!("Step 2: Found {} enums", enums.len());

        if let Some(first_enum) = enums.first() {
            eprintln!("Step 3: Enum name: {}", first_enum.node.ident);
            eprintln!("Step 4: Quoting enum...");
            let node = &first_enum.node;
            let tokens = quote::quote! { #node };
            eprintln!("Step 4: OK, quoted");

            eprintln!("Step 5: Parsing back to DeriveInput...");
            let _derive: syn::DeriveInput = syn::parse2(tokens)?;
            eprintln!("Step 5: OK");
        }

        eprintln!("Test completed successfully");

        Ok(())
    }
}
