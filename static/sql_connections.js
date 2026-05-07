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

        const editModal = document.getElementById('edit-connection-modal');
        const editClose = document.getElementById('edit-connection-close');
        const editCancel = document.getElementById('edit-connection-cancel');
        const editType = document.getElementById('edit_db_type');
        const editPgFields = document.getElementById('edit_pg_fields');
        const editHost = document.getElementById('edit_host');

        function toggleEditFields() {
            if (!editType || !editPgFields || !editHost) return;
            const isSqlite = editType.value === 'sqlite';
            editPgFields.style.display = isSqlite ? 'none' : 'block';
            editHost.placeholder = isSqlite ? 'File Path (e.g., ./my_data.db)' : 'Host (e.g., localhost:5432)';
        }

        function closeEditConnectionModal() {
            if (!editModal) return;
            editModal.hidden = true;
        }

        function openEditConnectionModal(button) {
            if (!editModal) return;
            document.getElementById('edit_original_nickname').value = button.dataset.nickname || '';
            document.getElementById('edit_nickname').value = button.dataset.nickname || '';
            document.getElementById('edit_db_type').value = button.dataset.dbType || 'postgres';
            document.getElementById('edit_host').value = button.dataset.host || '';
            document.getElementById('edit_db_name').value = button.dataset.dbName || '';
            document.getElementById('edit_user').value = button.dataset.user || '';
            document.getElementById('edit_password').value = '';
            toggleEditFields();
            editModal.hidden = false;
            document.getElementById('edit_nickname').focus();
        }

        document.querySelectorAll('.edit-connection-button').forEach((button) => {
            button.addEventListener('click', () => openEditConnectionModal(button));
        });
        editType?.addEventListener('change', toggleEditFields);
        editClose?.addEventListener('click', closeEditConnectionModal);
        editCancel?.addEventListener('click', closeEditConnectionModal);
        editModal?.addEventListener('click', (event) => {
            if (event.target === editModal) closeEditConnectionModal();
        });
        document.addEventListener('keydown', (event) => {
            if (event.key === 'Escape' && editModal && !editModal.hidden) {
                closeEditConnectionModal();
            }
        });
