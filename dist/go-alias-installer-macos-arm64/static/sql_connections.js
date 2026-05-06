        function toggleFields() {
            const type = document.getElementById('db_type').value;
            const pgFields = document.getElementById('pg_fields');
            const hostInput = document.getElementById('host_input');
            const inputs = pgFields.querySelectorAll('input');

            if (type === 'sqlite') {
                pgFields.style.display = 'none';
                hostInput.placeholder = "File Path (e.g., ./my_data.db)";
                inputs.forEach(i => i.removeAttribute('required'));
            } else {
                pgFields.style.display = 'block';
                hostInput.placeholder = "Host (e.g., localhost:5432)";
            }
        }
        
        function prepareCreate(e) {
            const input = document.getElementById('new_filename');
            let val = input.value.trim();
            if (!val) { e.preventDefault(); return; }
            
            // Auto append extension if missing
            if (!val.toLowerCase().endsWith('.db') && !val.toLowerCase().endsWith('.sqlite')) {
                val += '.db';
            }
            
            // Set hidden fields for the shared /sql/add endpoint
            document.getElementById('create_host').value = val;
            document.getElementById('create_nick').value = val;
        }

        // Run on load
        toggleFields();