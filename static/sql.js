const schemaTemplate = document.getElementById('sql-schema-data');
let dbSchema = schemaTemplate ? JSON.parse(schemaTemplate.textContent || '{}') : {};

const mainContent = document.getElementById('main');
      const editor = document.getElementById('sql-editor');
      const sidebarSearchInput = document.getElementById('sidebar-search-input');
      const sidebarTableList = document.getElementById('table-list');
      const querySearchInput = document.getElementById('query-search-input');
      const savedQueriesList = document.getElementById('saved-queries-list');
      const saveQueryForm = document.getElementById('save-query-form');
      const saveQueryBtn = document.getElementById('save-query-btn');
      const queryNameInput = document.getElementById('query-name');
      const querySqlInput = document.getElementById('query-sql');
      const queryFolderInput = document.getElementById('query-folder');
      const newSqlFileBtn = document.getElementById('new-sql-file-btn');
      const createQueryFolderBtn = document.getElementById('create-query-folder-btn');
      const createQueryFolderForm = document.getElementById('create-query-folder-form');
      const newQueryFolderName = document.getElementById('new-query-folder-name');
      const importQueryBtn = document.getElementById('import-query-btn');
      const importQueryFile = document.getElementById('import-query-file');
      const importQueryForm = document.getElementById('import-query-form');
      const importQueryPayload = document.getElementById('import-query-payload');
      const variablesSection = document.getElementById('variables-section');
      const variableHelpBtn = document.getElementById('variable-help-btn');
      const sqlDisconnectBtn = document.getElementById('sql-disconnect-btn');
      const autocompleteList = document.getElementById('autocomplete-list');
      const saveSqlFileBtn = document.getElementById('save-sql-file-btn');
      let currentFocus = -1;
      
      const backdrop = document.getElementById('sql-backdrop');
      const highlights = backdrop.querySelector('.highlights');
      const connectionNickname = document.querySelector("input[name=connection]")?.value || "";
      const varsStorageKey = "sql_vars_" + connectionNickname;
      const savedQueryFoldersCollapsedKey = "sql_saved_query_folders_collapsed_" + connectionNickname;
      let collapsedSqlFolders = readCollapsedSqlFolders();

      function openUntitledSqlFile() {
          editor.value = '';
          queryNameInput.value = '';
          if (queryFolderInput) {
              queryFolderInput.value = '';
          }
          scanForVariables();
          handleInput();
          editor.focus();
      }

      function readCollapsedSqlFolders() {
          try {
              const values = JSON.parse(localStorage.getItem(savedQueryFoldersCollapsedKey) || '[]');
              return new Set(Array.isArray(values) ? values : []);
          } catch (_) {
              return new Set();
          }
      }

      function saveCollapsedSqlFolders() {
          localStorage.setItem(savedQueryFoldersCollapsedKey, JSON.stringify(Array.from(collapsedSqlFolders)));
      }

      // --- Clear Button Logic ---
      const clearBtn = document.getElementById('clear-editor-btn');
      if (clearBtn) {
          clearBtn.addEventListener('click', () => {
              if (editor.value.trim() === '' && queryNameInput.value.trim() === '') return;
              openUntitledSqlFile();
          });
      }

      saveSqlFileBtn.addEventListener('click', () => {
          const content = editor.value;
          if (!content) return;
          
          const blob = new Blob([content], { type: 'text/plain' });
          const link = document.createElement('a');
          link.href = URL.createObjectURL(blob);
          
          let filename = queryNameInput.value.trim() || 'query';
          if (!filename.toLowerCase().endsWith('.sql')) filename += '.sql';
          
          link.download = filename;
          link.click();
          URL.revokeObjectURL(link.href);
      });

      if (saveQueryBtn && saveQueryForm) {
          saveQueryBtn.addEventListener('click', () => {
              if (queryNameInput.value.trim() === '') {
                  openSaveQueryDialog();
                  return;
              }

              if (typeof saveQueryForm.requestSubmit === 'function') {
                  saveQueryForm.requestSubmit();
                  return;
              }

              querySqlInput.value = editor.value;
              if (queryNameInput.value.trim() !== '') {
                  saveQueryForm.submit();
              }
          });
      }

      function submitSavedQuery() {
          if (typeof saveQueryForm.requestSubmit === 'function') {
              saveQueryForm.requestSubmit();
              return;
          }

          querySqlInput.value = editor.value;
          if (queryNameInput.value.trim() !== '') {
              saveQueryForm.submit();
          }
      }

      function closeSaveQueryDialog() {
          const existingDialog = document.getElementById('save-query-dialog-backdrop');
          if (existingDialog) {
              existingDialog.remove();
          }
      }

      function openSaveQueryDialog() {
          closeSaveQueryDialog();

          const backdropEl = document.createElement('div');
          backdropEl.id = 'save-query-dialog-backdrop';
          backdropEl.className = 'sql-dialog-backdrop';

          const dialogEl = document.createElement('div');
          dialogEl.className = 'sql-dialog';
          dialogEl.setAttribute('role', 'dialog');
          dialogEl.setAttribute('aria-modal', 'true');
          dialogEl.setAttribute('aria-labelledby', 'save-query-dialog-title');

          const titleEl = document.createElement('h3');
          titleEl.id = 'save-query-dialog-title';
          titleEl.textContent = 'Save Query';

          const nameLabel = document.createElement('label');
          nameLabel.textContent = 'Query name';
          const nameInput = document.createElement('input');
          nameInput.type = 'text';
          nameInput.placeholder = 'Name this query';
          nameInput.autocomplete = 'off';

          const folderLabel = document.createElement('label');
          folderLabel.textContent = 'Folder';
          const folderSelect = document.createElement('select');
          if (queryFolderInput) {
              Array.from(queryFolderInput.options).forEach((option) => {
                  folderSelect.appendChild(option.cloneNode(true));
              });
              folderSelect.value = queryFolderInput.value;
          } else {
              const option = document.createElement('option');
              option.value = '';
              option.textContent = 'Unfiled';
              folderSelect.appendChild(option);
          }

          const actionsEl = document.createElement('div');
          actionsEl.className = 'sql-dialog-actions';

          const cancelBtn = document.createElement('button');
          cancelBtn.type = 'button';
          cancelBtn.textContent = 'Cancel';
          cancelBtn.addEventListener('click', closeSaveQueryDialog);

          const saveBtn = document.createElement('button');
          saveBtn.type = 'button';
          saveBtn.textContent = 'Save';
          saveBtn.addEventListener('click', () => {
              const name = nameInput.value.trim();
              if (name === '') {
                  nameInput.focus();
                  return;
              }

              queryNameInput.value = name;
              if (queryFolderInput) {
                  queryFolderInput.value = folderSelect.value;
              }
              closeSaveQueryDialog();
              submitSavedQuery();
          });

          backdropEl.addEventListener('click', (event) => {
              if (event.target === backdropEl) {
                  closeSaveQueryDialog();
              }
          });

          dialogEl.addEventListener('keydown', (event) => {
              if (event.key === 'Escape') {
                  closeSaveQueryDialog();
              }
              if (event.key === 'Enter') {
                  event.preventDefault();
                  saveBtn.click();
              }
          });

          actionsEl.appendChild(cancelBtn);
          actionsEl.appendChild(saveBtn);
          dialogEl.appendChild(titleEl);
          dialogEl.appendChild(nameLabel);
          dialogEl.appendChild(nameInput);
          dialogEl.appendChild(folderLabel);
          dialogEl.appendChild(folderSelect);
          dialogEl.appendChild(actionsEl);
          backdropEl.appendChild(dialogEl);
          document.body.appendChild(backdropEl);
          nameInput.focus();
      }

      function closeVariableHelpDialog() {
          const existingDialog = document.getElementById('variable-help-dialog-backdrop');
          if (existingDialog) {
              existingDialog.remove();
          }
      }

      function openVariableHelpDialog() {
          closeVariableHelpDialog();

          const backdropEl = document.createElement('div');
          backdropEl.id = 'variable-help-dialog-backdrop';
          backdropEl.className = 'sql-dialog-backdrop';

          const dialogEl = document.createElement('div');
          dialogEl.className = 'sql-dialog sql-help-dialog';
          dialogEl.setAttribute('role', 'dialog');
          dialogEl.setAttribute('aria-modal', 'true');
          dialogEl.setAttribute('aria-labelledby', 'variable-help-dialog-title');

          const titleEl = document.createElement('h3');
          titleEl.id = 'variable-help-dialog-title';
          titleEl.textContent = 'SQL Variables';

          const bodyEl = document.createElement('div');
          bodyEl.className = 'sql-help-dialog-body';
          bodyEl.innerHTML = `
              <p>Use variables when a query needs values you may change each run.</p>
              <p>Type a placeholder in SQL using double braces, like <code>{{customer_id}}</code>. The variable bar will create an input for it automatically.</p>
              <p>Use <strong>+ Var</strong> to add a manual variable input, then use that same name in the query.</p>
              <pre>SELECT *
FROM orders
WHERE customer_id = {{customer_id}}
  AND status = '{{status}}';</pre>
          `;

          const actionsEl = document.createElement('div');
          actionsEl.className = 'sql-dialog-actions';

          const closeBtn = document.createElement('button');
          closeBtn.type = 'button';
          closeBtn.textContent = 'Close';
          closeBtn.addEventListener('click', closeVariableHelpDialog);

          backdropEl.addEventListener('click', (event) => {
              if (event.target === backdropEl) {
                  closeVariableHelpDialog();
              }
          });

          dialogEl.addEventListener('keydown', (event) => {
              if (event.key === 'Escape') {
                  closeVariableHelpDialog();
              }
          });

          actionsEl.appendChild(closeBtn);
          dialogEl.appendChild(titleEl);
          dialogEl.appendChild(bodyEl);
          dialogEl.appendChild(actionsEl);
          backdropEl.appendChild(dialogEl);
          document.body.appendChild(backdropEl);
          closeBtn.focus();
      }

      if (variableHelpBtn) {
          variableHelpBtn.addEventListener('click', openVariableHelpDialog);
      }

      if (sqlDisconnectBtn) {
          sqlDisconnectBtn.addEventListener('click', async () => {
              if (typeof window.closeSqlConnectionTab === 'function') {
                  await window.closeSqlConnectionTab(connectionNickname);
                  return;
              }

              try {
                  await fetch(`/sql/disconnect/${encodeURIComponent(connectionNickname)}`, { method: 'POST' });
              } catch (error) {
                  console.error('Failed to disconnect SQL connection', error);
              }
              window.location.href = '/sql';
          });
      }
      
      const outputResizer = document.getElementById('output-resizer');
      const outputPane = document.getElementById('output');
      let isOutputResizing = false;
      let lastOutputDownY = 0;
      let startOutputHeight = 0;

      outputResizer.addEventListener('mousedown', (e) => {
          isOutputResizing = true;
          lastOutputDownY = e.clientY;
          startOutputHeight = outputPane.offsetHeight;
          outputResizer.classList.add('resizing');
          document.body.style.cursor = 'row-resize';
          document.body.style.userSelect = 'none';
      });

      document.addEventListener('mousemove', (e) => {
          if (isOutputResizing) {
              const dy = lastOutputDownY - e.clientY;
              let newHeight = startOutputHeight + dy;
              if (newHeight < 50) newHeight = 50; 
              
              const containerHeight = mainContent.clientHeight;
              if (newHeight > containerHeight - 100) newHeight = containerHeight - 100;
              
              outputPane.style.height = newHeight + 'px';
          }
      });

      document.addEventListener('mouseup', (e) => {
          if (isOutputResizing) {
              isOutputResizing = false;
              outputResizer.classList.remove('resizing');
              document.body.style.cursor = '';
              document.body.style.userSelect = '';
          }
      });

      const tableQueryResizer = document.getElementById('table-query-resizer');
      const tableListPane = document.getElementById('table-list');
      const tableListHeightKey = `sql_table_list_height_${connectionNickname}`;
      let isTableListResizing = false;
      let lastTableListDownY = 0;
      let startTableListHeight = 0;

      const savedTableListHeight = Number(localStorage.getItem(tableListHeightKey));
      if (savedTableListHeight > 40) {
          tableListPane.style.flex = '0 0 auto';
          tableListPane.style.height = `${savedTableListHeight}px`;
      }

      if (tableQueryResizer && tableListPane) {
          tableQueryResizer.addEventListener('mousedown', (event) => {
              isTableListResizing = true;
              lastTableListDownY = event.clientY;
              startTableListHeight = tableListPane.offsetHeight;
              tableQueryResizer.classList.add('resizing');
              document.body.style.cursor = 'row-resize';
              document.body.style.userSelect = 'none';
          });
      }

      document.addEventListener('mousemove', (event) => {
          if (!isTableListResizing) return;

          const delta = event.clientY - lastTableListDownY;
          const sidebarHeight = document.getElementById('sidebar')?.clientHeight || 300;
          const maxHeight = Math.max(80, sidebarHeight - 170);
          let nextHeight = startTableListHeight + delta;
          if (nextHeight < 40) nextHeight = 40;
          if (nextHeight > maxHeight) nextHeight = maxHeight;

          tableListPane.style.flex = '0 0 auto';
          tableListPane.style.height = `${nextHeight}px`;
      });

      document.addEventListener('mouseup', () => {
          if (!isTableListResizing) return;

          isTableListResizing = false;
          tableQueryResizer.classList.remove('resizing');
          document.body.style.cursor = '';
          document.body.style.userSelect = '';
          localStorage.setItem(tableListHeightKey, tableListPane.offsetHeight.toString());
      });

      function renderTableList() {
          const filter = sidebarSearchInput.value.toUpperCase();
          sidebarTableList.innerHTML = '';
          
          Object.keys(dbSchema).sort().forEach(tableName => {
              if (tableName.toUpperCase().indexOf(filter) > -1) {
                  const li = document.createElement('li');
                  const a = document.createElement('a');
                  a.className = 'table-list-item';
                  a.textContent = tableName;
                  a.href = '#';
                  a.title = "Click to SELECT * LIMIT 100";
                  a.onclick = (e) => { e.preventDefault(); editor.value = "SELECT * FROM " + tableName + " LIMIT 100;"; handleInput(); };
                  li.appendChild(a);
                  sidebarTableList.appendChild(li);
              }
          });
      }
      renderTableList();
      sidebarSearchInput.addEventListener('keyup', renderTableList);

      const refreshBtn = document.getElementById('refresh-schema-btn');
      refreshBtn.addEventListener('click', refreshSchema);

      async function refreshSchema() {
          refreshBtn.style.animation = "spin 1s linear infinite";
          try {
              const resp = await fetch('/sql/' + connectionNickname + '/schema-json');
              if (resp.ok) {
                  dbSchema = await resp.json();
                  renderTableList();
              } else {
                  console.error("Failed to refresh schema");
              }
          } catch(e) {
              console.error(e);
          } finally {
              refreshBtn.style.animation = "none";
          }
      }
      
      // Inject spin animation
      const styleSheet = document.createElement("style");
      styleSheet.innerText = `@keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }`;
      document.head.appendChild(styleSheet);

      function normalizeFolderPath(folder) {
          return String(folder || '')
              .replace(/\s+\/\s+/g, '/')
              .split('/')
              .map((part) => part.trim())
              .filter(Boolean)
              .join('/');
      }

      function isSameOrChildFolder(folder, parent) {
          return folder === parent || folder.startsWith(parent + '/');
      }

      function pathHasCollapsedFolder(folder, collapsedFolders, includeSelf = true) {
          const normalized = normalizeFolderPath(folder);
          if (!normalized) return false;
          const parts = normalized.split('/');
          const max = includeSelf ? parts.length : parts.length - 1;
          for (let index = 1; index <= max; index += 1) {
              if (collapsedFolders.has(parts.slice(0, index).join('/'))) {
                  return true;
              }
          }
          return false;
      }

      function writeSqlDragPayload(event, payload) {
          const raw = JSON.stringify(payload);
          event.dataTransfer.clearData();
          event.dataTransfer.setData('application/x-go-sql-drag', raw);
          event.dataTransfer.setData('text/plain', raw);
          event.dataTransfer.effectAllowed = 'move';
      }

      function readSqlDragPayload(event) {
          const raw = event.dataTransfer.getData('application/x-go-sql-drag')
              || event.dataTransfer.getData('text/plain')
              || '{}';
          return JSON.parse(raw);
      }

      function clearSqlDropTargets() {
          savedQueriesList.querySelectorAll('.dragging, .drop-target, .drop-target-invalid').forEach((element) => {
              element.classList.remove('dragging', 'drop-target', 'drop-target-invalid');
          });
          savedQueriesList.classList.remove('drop-target', 'drop-target-invalid');
      }

      async function postSqlMove(url, fields) {
          const body = new URLSearchParams();
          Object.entries(fields).forEach(([key, value]) => body.append(key, value || ''));
          const response = await fetch(url, {
              method: 'POST',
              headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' },
              body,
          });
          if (!response.ok) {
              throw new Error(await response.text());
          }
      }

      function filterSavedQueries() {
          const filter = querySearchInput.value.toUpperCase();
          const queryItems = Array.from(savedQueriesList.querySelectorAll('.saved-query-item'));
          const visibleFolders = new Set();

          queryItems.forEach((item) => {
              const queryLink = item.querySelector('.query-link');
              if (!queryLink) return;

              const folder = normalizeFolderPath(item.dataset.folder || queryLink.dataset.folder || '');
              const itemText = queryLink.textContent || queryLink.innerText || '';
              const isCollapsed = pathHasCollapsedFolder(folder, collapsedSqlFolders, true);
              const isVisible = itemText.toUpperCase().indexOf(filter) > -1 && !isCollapsed;
              item.style.display = isVisible ? 'flex' : 'none';

              if (itemText.toUpperCase().indexOf(filter) > -1 && folder) {
                  const parts = folder.split('/');
                  for (let index = 1; index <= parts.length; index += 1) {
                      visibleFolders.add(parts.slice(0, index).join('/'));
                  }
              }
          });

          Array.from(savedQueriesList.querySelectorAll('.saved-query-folder')).forEach((folder) => {
              const folderPath = normalizeFolderPath(folder.dataset.folder || '');
              const folderText = folder.textContent || '';
              const matchesFolder = folderPath.toUpperCase().includes(filter) || folderText.toUpperCase().includes(filter);
              const hiddenByAncestor = pathHasCollapsedFolder(folderPath, collapsedSqlFolders, false);
              const isCollapsed = collapsedSqlFolders.has(folderPath);
              const shouldShow = !hiddenByAncestor && (folderPath === '' || filter === '' || visibleFolders.has(folderPath) || matchesFolder);
              folder.style.display = shouldShow ? 'block' : 'none';
              folder.classList.toggle('collapsed', isCollapsed);
              const toggle = folder.querySelector('.saved-query-folder-toggle');
              if (toggle) toggle.textContent = isCollapsed ? '▸' : '▾';
          });
      }
      querySearchInput.addEventListener('keyup', filterSavedQueries);

      if (createQueryFolderBtn && createQueryFolderForm && newQueryFolderName) {
          createQueryFolderBtn.addEventListener('click', () => {
              const folderName = window.prompt('Folder name');
              if (!folderName || folderName.trim() === '') return;

              newQueryFolderName.value = normalizeFolderPath(folderName);
              createQueryFolderForm.submit();
          });
      }

      if (savedQueriesList) {
          savedQueriesList.addEventListener('dragstart', (event) => {
              const folder = event.target.closest('.saved-query-folder[data-folder]');
              const item = event.target.closest('.saved-query-item');

              if (folder && folder.dataset.folder) {
                  writeSqlDragPayload(event, {
                      type: 'folder',
                      folder: normalizeFolderPath(folder.dataset.folder),
                  });
                  folder.classList.add('dragging');
                  return;
              }

              if (item) {
                  writeSqlDragPayload(event, {
                      type: 'query',
                      name: item.dataset.queryName || '',
                      folder: normalizeFolderPath(item.dataset.folder || ''),
                  });
                  item.classList.add('dragging');
              }
          });

          savedQueriesList.addEventListener('dragend', () => {
              clearSqlDropTargets();
          });

          savedQueriesList.addEventListener('dragover', (event) => {
              event.preventDefault();
              event.dataTransfer.dropEffect = 'move';
              savedQueriesList.querySelectorAll('.drop-target, .drop-target-invalid').forEach((element) => {
                  element.classList.remove('drop-target', 'drop-target-invalid');
              });
              savedQueriesList.classList.remove('drop-target', 'drop-target-invalid');

              const folder = event.target.closest('.saved-query-folder[data-folder]');
              let payload = {};
              try {
                  payload = readSqlDragPayload(event);
              } catch {
                  payload = {};
              }

              if (folder) {
                  const targetFolder = normalizeFolderPath(folder.dataset.folder || '');
                  const draggedFolder = normalizeFolderPath(payload.folder || '');
                  const invalid = payload.type === 'folder' && (!draggedFolder || draggedFolder === targetFolder || isSameOrChildFolder(targetFolder, draggedFolder));
                  folder.classList.add(invalid ? 'drop-target-invalid' : 'drop-target');
                  event.dataTransfer.dropEffect = invalid ? 'none' : 'move';
              } else {
                  savedQueriesList.classList.add('drop-target');
              }
          });

          savedQueriesList.addEventListener('dragleave', (event) => {
              const target = event.target.closest('.saved-query-folder');
              if (target) target.classList.remove('drop-target', 'drop-target-invalid');
              if (!savedQueriesList.contains(event.relatedTarget)) {
                  savedQueriesList.classList.remove('drop-target', 'drop-target-invalid');
              }
          });

          savedQueriesList.addEventListener('drop', async (event) => {
              event.preventDefault();

              let payload;
              try {
                  payload = readSqlDragPayload(event);
              } catch {
                  return;
              } finally {
                  clearSqlDropTargets();
              }

              const targetFolder = normalizeFolderPath(event.target.closest('.saved-query-folder[data-folder]')?.dataset.folder || '');
              try {
                  if (payload.type === 'query') {
                      await postSqlMove('/sql/query/move', {
                          query_name: payload.name,
                          connection: connectionNickname,
                          new_folder: targetFolder,
                      });
                      window.location.reload();
                  } else if (payload.type === 'folder') {
                      const draggedFolder = normalizeFolderPath(payload.folder);
                      if (!draggedFolder || draggedFolder === targetFolder || isSameOrChildFolder(targetFolder, draggedFolder)) return;
                      await postSqlMove('/sql/folder/move', {
                          folder_name: draggedFolder,
                          connection: connectionNickname,
                          new_parent: targetFolder,
                      });
                      window.location.reload();
                  }
              } catch (error) {
                  window.alert(`Move failed: ${error.message}`);
              }
          });
      }

      if (newSqlFileBtn) {
          newSqlFileBtn.addEventListener('click', () => {
              openUntitledSqlFile();
          });
      }

      if (importQueryBtn && importQueryFile && importQueryForm && importQueryPayload) {
          importQueryBtn.addEventListener('click', () => {
              importQueryFile.value = '';
              importQueryFile.click();
          });

          importQueryFile.addEventListener('change', async () => {
              const file = importQueryFile.files && importQueryFile.files[0];
              if (!file) return;

              try {
                  const payload = await file.text();
                  JSON.parse(payload);
                  if (!window.confirm('Import saved queries into this connection? Duplicate names will be renamed.')) {
                      return;
                  }
                  importQueryPayload.value = payload;
                  importQueryForm.submit();
              } catch (error) {
                  window.alert('That file does not look like valid JSON.');
              }
          });
      }

      const form = document.getElementById('sql-form');
      const output = document.getElementById('output');
      const outputFilterInput = document.getElementById('output-filter');
      const outputHistorySelect = document.getElementById('output-history-select');
      const deleteOutputHistoryBtn = document.getElementById('delete-output-history-btn');
      const clearOutputHistoryBtn = document.getElementById('clear-output-history-btn');
      const sqlJobsSelect = document.getElementById('sql-jobs-select');
      const columnMenuBtn = document.getElementById('column-menu-btn');
      const columnMenuPanel = document.getElementById('column-menu-panel');
      const exportMenuBtn = document.getElementById('export-menu-btn');
      const exportMenuPanel = document.getElementById('export-menu-panel');
      const outputHistoryStorageKey = "sql_output_history_" + connectionNickname;
      const maxOutputHistoryEntries = 8;
      const maxOutputHistoryEntryChars = 500000;
      const maxOutputHistoryTotalChars = 1500000;
      let activeSqlJobId = '';
      let sqlJobPollTimer = null;

      function loadOutputHistory() {
          try {
              const history = JSON.parse(localStorage.getItem(outputHistoryStorageKey) || '[]');
              if (!Array.isArray(history)) return [];

              return history.map((entry, index) => ({
                  ...entry,
                  id: entry.id || `${entry.createdAt || 'history'}-${index}`
              }));
          } catch (e) {
              console.error('Failed to load SQL output history', e);
              return [];
          }
      }

      function saveOutputHistory(history) {
          try {
              localStorage.setItem(outputHistoryStorageKey, JSON.stringify(history));
          } catch (e) {
              console.error('Failed to save SQL output history', e);
          }
      }

      function getOutputRowCount() {
          const table = output.querySelector('table');
          if (!table) return '';

          const rows = table.querySelectorAll('tbody tr');
          return rows.length + " rows";
      }

      function applyOutputHtml(html, rowCountText = '') {
          output.innerHTML = html;
          if (outputFilterInput) {
              outputFilterInput.value = '';
          }

          const table = output.querySelector('table');
          if (table) {
              makeTableInteractable(table);
          }

          const countSpan = document.getElementById('row-count');
          if (countSpan) {
              countSpan.innerText = rowCountText || getOutputRowCount();
          }
          updateSelectionCount();
      }

      function outputHistoryLabel(entry) {
          const date = new Date(entry.createdAt);
          const time = Number.isNaN(date.getTime()) ? '' : date.toLocaleString();
          const name = entry.queryName || entry.sql.replace(/\s+/g, ' ').trim().slice(0, 64) || 'SQL output';
          const rows = entry.rowCountText ? ` - ${entry.rowCountText}` : '';
          return `${time} - ${name}${rows}`;
      }

      function renderOutputHistoryOptions(selectedId = '') {
          if (!outputHistorySelect) return;

          const nextSelectedId = selectedId || outputHistorySelect.value;
          const history = loadOutputHistory();
          outputHistorySelect.innerHTML = '<option value="">Output history</option>';
          history.forEach((entry) => {
              const option = document.createElement('option');
              option.value = entry.id;
              option.textContent = outputHistoryLabel(entry);
              outputHistorySelect.appendChild(option);
          });
          if (nextSelectedId && history.some((entry) => entry.id === nextSelectedId)) {
              outputHistorySelect.value = nextSelectedId;
          }
      }

      function pruneOutputHistory(history) {
          let pruned = history.slice(0, maxOutputHistoryEntries);
          let totalChars = 0;
          pruned = pruned.filter((entry) => {
              totalChars += entry.html.length;
              return totalChars <= maxOutputHistoryTotalChars;
          });
          return pruned;
      }

      function cacheOutputHistory(html, sql) {
          if (!html || html.length > maxOutputHistoryEntryChars) return;

          const entry = {
              id: String(Date.now()) + "-" + Math.random().toString(16).slice(2),
              createdAt: new Date().toISOString(),
              sql: sql,
              queryName: queryNameInput.value.trim(),
              queryFolder: queryFolderInput ? queryFolderInput.value : '',
              rowCountText: getOutputRowCount(),
              html: html
          };

          const history = loadOutputHistory().filter((existing) => existing.html !== html || existing.sql !== sql);
          history.unshift(entry);
          saveOutputHistory(pruneOutputHistory(history));
          renderOutputHistoryOptions(entry.id);
      }

      function sqlJobLabel(job) {
          const status = job.status === 'running' ? 'Running' : 'Done';
          const date = job.completed_at || job.created_at || '';
          const name = (job.query_name || job.sql || 'SQL job').replace(/\s+/g, ' ').trim().slice(0, 58);
          const rows = job.row_count_text ? ` - ${job.row_count_text}` : '';
          return `${status} - ${date} - ${name}${rows}`;
      }

      function renderSqlJobs(jobs = [], selectedId = '') {
          if (!sqlJobsSelect) return;
          const nextSelectedId = selectedId || sqlJobsSelect.value;
          sqlJobsSelect.innerHTML = '<option value="">Running queries</option>';
          jobs.forEach((job) => {
              const option = document.createElement('option');
              option.value = job.id;
              option.dataset.status = job.status;
              option.textContent = sqlJobLabel(job);
              sqlJobsSelect.appendChild(option);
          });
          if (nextSelectedId && jobs.some((job) => job.id === nextSelectedId)) {
              sqlJobsSelect.value = nextSelectedId;
          }
      }

      async function refreshSqlJobs(selectedId = '') {
          if (!sqlJobsSelect || !connectionNickname) return [];
          try {
              const resp = await fetch(`/sql/jobs/${encodeURIComponent(connectionNickname)}`);
              if (!resp.ok) throw new Error(await resp.text());
              const jobs = await resp.json();
              renderSqlJobs(jobs, selectedId);
              return jobs;
          } catch (err) {
              console.error('Failed to load SQL jobs', err);
              return [];
          }
      }

      async function applyCompletedSqlJob(job) {
          if (!job || job.status !== 'completed' || !job.html) return;
          applyOutputHtml(job.html, job.row_count_text || '');
          editor.value = job.sql || editor.value;
          queryNameInput.value = job.query_name || '';
          if (queryFolderInput) queryFolderInput.value = job.query_folder || '';
          scanForVariables();
          handleInput();
          cacheOutputHistory(job.html, job.sql || editor.value);
          await fetch(`/sql/job/${encodeURIComponent(job.id)}/activate`, { method: 'POST' }).catch(() => {});
      }

      async function fetchSqlJob(jobId) {
          const resp = await fetch(`/sql/job/${encodeURIComponent(jobId)}`);
          if (!resp.ok) throw new Error(await resp.text());
          return resp.json();
      }

      function stopSqlJobPolling() {
          if (sqlJobPollTimer) {
              window.clearTimeout(sqlJobPollTimer);
              sqlJobPollTimer = null;
          }
      }

      function pollSqlJob(jobId) {
          stopSqlJobPolling();
          activeSqlJobId = jobId;
          const tick = async () => {
              try {
                  const job = await fetchSqlJob(jobId);
                  await refreshSqlJobs(jobId);
                  if (job.status === 'completed') {
                      output.innerHTML = '<pre style="padding:10px;">Query completed. Loading output...</pre>';
                      await applyCompletedSqlJob(job);
                      activeSqlJobId = '';
                      stopSqlJobPolling();
                      return;
                  }
                  output.innerHTML = `<pre style="padding:10px;">Query is still running in the background...\nStarted: ${job.created_at || ''}</pre>`;
                  sqlJobPollTimer = window.setTimeout(tick, 1500);
              } catch (err) {
                  output.innerHTML = '<pre style="padding:10px; color:#ff6b6b;">Job status error: ' + err.message + '</pre>';
                  activeSqlJobId = '';
                  stopSqlJobPolling();
              }
          };
          tick();
      }

      function getSelectedOutputHistoryEntry(id = '') {
          const selectedId = id || outputHistorySelect?.value || '';
          if (selectedId === '') return null;

          return loadOutputHistory().find((entry) => entry.id === selectedId) || null;
      }

      renderOutputHistoryOptions();

      function restoreSelectedOutputHistory() {
          const entry = getSelectedOutputHistoryEntry();
          if (!entry) return;

          applyOutputHtml(entry.html, entry.rowCountText);
          editor.value = entry.sql || editor.value;
          queryNameInput.value = entry.queryName || '';
          if (queryFolderInput) {
              queryFolderInput.value = entry.queryFolder || '';
          }
          scanForVariables();
          handleInput();
      }

      if (outputHistorySelect) {
          outputHistorySelect.addEventListener('input', restoreSelectedOutputHistory);
          outputHistorySelect.addEventListener('change', restoreSelectedOutputHistory);
      }

      if (deleteOutputHistoryBtn) {
          deleteOutputHistoryBtn.addEventListener('click', () => {
              if (!outputHistorySelect || outputHistorySelect.value === '') return;
              if (!window.confirm('Delete the selected SQL output history entry?')) return;

              const nextHistory = loadOutputHistory().filter((entry) => entry.id !== outputHistorySelect.value);
              saveOutputHistory(nextHistory);
              renderOutputHistoryOptions();
          });
      }

      if (clearOutputHistoryBtn) {
          clearOutputHistoryBtn.addEventListener('click', () => {
              if (!window.confirm('Clear all cached SQL output history for this connection?')) return;

              saveOutputHistory([]);
              renderOutputHistoryOptions();
          });
      }

      refreshSqlJobs();
      if (sqlJobsSelect) {
          sqlJobsSelect.addEventListener('change', async () => {
              if (!sqlJobsSelect.value) return;
              try {
                  const job = await fetchSqlJob(sqlJobsSelect.value);
                  if (job.status === 'completed') {
                      await applyCompletedSqlJob(job);
                  } else {
                      pollSqlJob(job.id);
                  }
              } catch (err) {
                  output.innerHTML = '<pre style="padding:10px; color:#ff6b6b;">Job load error: ' + err.message + '</pre>';
              }
          });
      }
      
      form.addEventListener('submit', async (e) => {
        e.preventDefault();
        output.innerHTML = '<pre style="padding:10px;">Starting background query...</pre>';
        
        const variables = {};
        const varInputs = variablesSection.querySelectorAll('input');
        varInputs.forEach(input => {
            if(input.name && input.value) {
                variables[input.name] = input.value;
            }
        });

        const payload = {
            sql: editor.value,
            connection: form.querySelector('input[name="connection"]').value,
            variables: variables
        };

        try {
            const resp = await fetch('/sql/run-background', {
                method: 'POST', 
                headers: { 'Content-Type': 'application/json' }, 
                body: JSON.stringify(payload) 
            });
            if (!resp.ok) throw new Error(await resp.text());
            const result = await resp.json();
            await refreshSqlJobs(result.job_id);
            pollSqlJob(result.job_id);

            // AUTO-REFRESH SCHEMA on DDL
            const upperSql = payload.sql.toUpperCase();
            if (upperSql.includes("CREATE TABLE") || 
                upperSql.includes("DROP TABLE") || 
                upperSql.includes("ALTER TABLE")) {
                refreshSchema();
            }

        } catch(e) {
            output.innerHTML = '<pre style="padding:10px; color:#ff6b6b;">Error: ' + e.message + '</pre>';
        }
      });

      document.addEventListener('keydown', (event) => {
          if (!(event.metaKey || event.ctrlKey) || event.key !== 'Enter') return;
          event.preventDefault();

          if (typeof form.requestSubmit === 'function') {
              form.requestSubmit();
          } else {
              form.dispatchEvent(new Event('submit', { cancelable: true }));
          }
      });
      
      function setMenuOpen(button, panel, isOpen) {
          if (!button || !panel) return;
          panel.hidden = !isOpen;
          button.setAttribute('aria-expanded', String(isOpen));
      }

      function toggleMenu(button, panel) {
          const wasOpen = panel && !panel.hidden;
          setMenuOpen(columnMenuBtn, columnMenuPanel, false);
          setMenuOpen(exportMenuBtn, exportMenuPanel, false);
          setMenuOpen(button, panel, !wasOpen);
      }

      if (columnMenuBtn && columnMenuPanel) {
          columnMenuBtn.addEventListener('click', (event) => {
              event.stopPropagation();
              toggleMenu(columnMenuBtn, columnMenuPanel);
          });
      }

      if (exportMenuBtn && exportMenuPanel) {
          exportMenuBtn.addEventListener('click', (event) => {
              event.stopPropagation();
              toggleMenu(exportMenuBtn, exportMenuPanel);
          });
      }

      document.addEventListener('click', (event) => {
          if (!event.target.closest('.sql-result-menu')) {
              setMenuOpen(columnMenuBtn, columnMenuPanel, false);
              setMenuOpen(exportMenuBtn, exportMenuPanel, false);
          }
      });

      function csvEscape(value) {
          return `"${String(value || '').replace(/"/g, '""')}"`;
      }

      function getCleanHeaderText(th, fallback = '') {
          return th?.childNodes[0]?.textContent?.trim() || fallback;
      }

      function getVisibleColumnIndexes(table) {
          return Array.from(table.querySelectorAll('thead th'))
              .map((th, index) => th.classList.contains('sql-column-hidden') ? -1 : index)
              .filter((index) => index >= 0);
      }

      function getExportRows(table, rowsMode) {
          const visibleBodyRows = Array.from(table.querySelectorAll('tbody tr'))
              .filter((row) => row.style.display !== 'none');
          if (rowsMode === 'selected') {
              return visibleBodyRows.filter((row) => row.classList.contains('selected-row'));
          }
          return visibleBodyRows;
      }

      function exportCurrentResults(mode) {
          const table = output.querySelector('table');
          if (!table) return alert('No results to export');

          const includeHeaders = mode.endsWith('-headers');
          const rowsMode = mode.startsWith('selected') ? 'selected' : 'all';
          const visibleColumnIndexes = getVisibleColumnIndexes(table);
          if (visibleColumnIndexes.length === 0) return alert('No visible columns to export');

          const rowsToExport = getExportRows(table, rowsMode);
          if (rowsMode === 'selected' && rowsToExport.length === 0) {
              return alert('No selected rows to export');
          }

          const theadCells = Array.from(table.querySelectorAll('thead th'));
          let csvContent = "";

          if (includeHeaders && theadCells.length > 0) {
              csvContent += visibleColumnIndexes
                  .map((index) => csvEscape(getCleanHeaderText(theadCells[index], `Column ${index + 1}`)))
                  .join(",") + "\n";
          }

          rowsToExport.forEach(row => {
              const cells = Array.from(row.children);
              csvContent += visibleColumnIndexes
                  .map((index) => csvEscape(cells[index]?.innerText || ''))
                  .join(",") + "\n";
          });

          const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
          const link = document.createElement("a");
          const url = URL.createObjectURL(blob);
          link.setAttribute("href", url);
          link.setAttribute("download", `${connectionNickname || 'sql'}-${rowsMode}-results.csv`);
          link.style.visibility = 'hidden';
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
          URL.revokeObjectURL(url);
      }

      if (exportMenuPanel) {
          exportMenuPanel.addEventListener('click', (event) => {
              const button = event.target.closest('[data-export-mode]');
              if (!button) return;
              exportCurrentResults(button.dataset.exportMode || 'all-headers');
              setMenuOpen(exportMenuBtn, exportMenuPanel, false);
          });
      }
      
      // Filtering Logic
      outputFilterInput.addEventListener('input', (e) => {
          const term = e.target.value.toLowerCase();
          const table = output.querySelector('table');
          if(!table) return;
          const rows = table.querySelectorAll('tbody tr');
          
          let visibleCount = 0;
          rows.forEach(row => {
              const text = row.innerText.toLowerCase();
              if (text.includes(term)) {
                  row.style.display = '';
                  visibleCount++;
              } else {
                  row.style.display = 'none';
              }
          });
          
          const countSpan = document.getElementById('row-count');
          if(countSpan) countSpan.innerText = visibleCount + " rows";
      });

      function updateSelectionCount() {
          const selected = document.querySelectorAll('.output tr.selected-row').length;
          const btn = document.getElementById('clear-selection-btn');
          if (btn) {
              if (selected > 0) {
                  btn.style.display = 'inline-block';
                  btn.innerText = `Clear (${selected})`;
              } else {
                  btn.style.display = 'none';
              }
          }
      }

      const clearSelectionBtn = document.getElementById('clear-selection-btn');
      if (clearSelectionBtn) {
          clearSelectionBtn.addEventListener('click', () => {
              document.querySelectorAll('.output tr.selected-row').forEach(row => row.classList.remove('selected-row'));
              updateSelectionCount();
          });
      }

      function makeTableInteractable(table) {
        const ths = table.querySelectorAll('th');
        const tbody = table.querySelector('tbody');
        if (!tbody) return;
        const rows = Array.from(tbody.querySelectorAll('tr'));
        const minColumnWidth = 36;

        table.classList.add('resizable-output-table');

        let colgroup = table.querySelector('colgroup');
        if (!colgroup) {
            colgroup = document.createElement('colgroup');
            ths.forEach((th) => {
                const col = document.createElement('col');
                col.style.width = Math.max(th.offsetWidth, minColumnWidth) + 'px';
                colgroup.appendChild(col);
            });
            table.insertBefore(colgroup, table.firstChild);
        }
        
        rows.forEach((row, i) => {
            row.dataset.originalIndex = i;
        });

        tbody.addEventListener('click', (e) => {
            const tr = e.target.closest('tr');
            if (!tr) return;
            if (e.target.closest('button, input, textarea, select, a')) return;
            if (window.getSelection && window.getSelection().toString().trim() !== '') return;

            tr.classList.toggle('selected-row');
            updateSelectionCount();
        });

        let currentSortCol = -1;
        let currentSortDir = 'none'; 

        ths.forEach((th, colIndex) => {
            th.dataset.columnIndex = String(colIndex);
            const sortIndicator = document.createElement('span');
            sortIndicator.className = 'column-sort-indicator';
            th.appendChild(sortIndicator);

            const handle = document.createElement('span');
            handle.className = 'column-resize-handle';
            handle.title = 'Drag to resize column';
            th.appendChild(handle);

            handle.addEventListener('mousedown', (event) => {
                event.preventDefault();
                event.stopPropagation();

                const col = colgroup.children[colIndex];
                const startX = event.clientX;
                const startWidth = col?.getBoundingClientRect().width || th.getBoundingClientRect().width;
                handle.classList.add('resizing');
                document.body.style.cursor = 'col-resize';
                document.body.style.userSelect = 'none';

                const onMouseMove = (moveEvent) => {
                    const nextWidth = Math.max(minColumnWidth, startWidth + moveEvent.clientX - startX);
                    if (col) {
                        col.style.width = nextWidth + 'px';
                    }
                };

                const onMouseUp = () => {
                    handle.classList.remove('resizing');
                    document.body.style.cursor = '';
                    document.body.style.userSelect = '';
                    document.removeEventListener('mousemove', onMouseMove);
                    document.removeEventListener('mouseup', onMouseUp);
                };

                document.addEventListener('mousemove', onMouseMove);
                document.addEventListener('mouseup', onMouseUp);
            });

            th.addEventListener('click', () => {
                if (currentSortCol === colIndex) {
                    if (currentSortDir === 'none') currentSortDir = 'asc';
                    else if (currentSortDir === 'asc') currentSortDir = 'desc';
                    else currentSortDir = 'none';
                } else {
                    currentSortCol = colIndex;
                    currentSortDir = 'asc';
                }

                ths.forEach(h => {
                    const indicator = h.querySelector('.column-sort-indicator');
                    if (indicator) indicator.textContent = '';
                });
                if (currentSortDir === 'asc') sortIndicator.textContent = ' ▲';
                if (currentSortDir === 'desc') sortIndicator.textContent = ' ▼';

                const newRows = Array.from(rows);
                if (currentSortDir !== 'none') {
                    newRows.sort((rowA, rowB) => {
                        const cellA = rowA.children[colIndex].innerText.trim();
                        const cellB = rowB.children[colIndex].innerText.trim();
                        
                        const numA = parseFloat(cellA.replace(/[$,]/g, ''));
                        const numB = parseFloat(cellB.replace(/[$,]/g, ''));
                        
                        let comparison = 0;
                        if (!isNaN(numA) && !isNaN(numB) && !/[a-zA-Z]/.test(cellA) && !/[a-zA-Z]/.test(cellB)) {
                            comparison = numA - numB;
                        } else {
                            comparison = cellA.localeCompare(cellB, undefined, { numeric: true, sensitivity: 'base' });
                        }
                        
                        return currentSortDir === 'asc' ? comparison : -comparison;
                    });
                } else {
                    newRows.sort((a, b) => a.dataset.originalIndex - b.dataset.originalIndex);
                }

                tbody.innerHTML = '';
                newRows.forEach(row => tbody.appendChild(row));
                
                // Re-apply filter after sort
                document.getElementById('output-filter').dispatchEvent(new Event('input'));
            });
        });

        renderColumnMenu(table);
      }

      function setColumnVisibility(table, colIndex, isVisible) {
          const cells = table.querySelectorAll(`thead th:nth-child(${colIndex + 1}), tbody td:nth-child(${colIndex + 1})`);
          cells.forEach((cell) => {
              cell.classList.toggle('sql-column-hidden', !isVisible);
          });
      }

      function renderColumnMenu(table) {
          if (!columnMenuPanel) return;

          const headers = Array.from(table.querySelectorAll('thead th'));
          columnMenuPanel.innerHTML = '';

          if (headers.length === 0) {
              const empty = document.createElement('div');
              empty.className = 'sql-result-menu-empty';
              empty.textContent = 'No columns available.';
              columnMenuPanel.appendChild(empty);
              return;
          }

          headers.forEach((th, colIndex) => {
              const label = document.createElement('label');
              label.className = 'sql-column-menu-item';

              const checkbox = document.createElement('input');
              checkbox.type = 'checkbox';
              checkbox.checked = !th.classList.contains('sql-column-hidden');
              checkbox.addEventListener('change', () => {
                  setColumnVisibility(table, colIndex, checkbox.checked);
              });

              const labelText = document.createElement('span');
              labelText.textContent = getCleanHeaderText(th, `Column ${colIndex + 1}`);

              label.appendChild(checkbox);
              label.appendChild(labelText);
              columnMenuPanel.appendChild(label);
          });
      }

      savedQueriesList.addEventListener('click', (e) => {
          const renameButton = e.target.closest('.rename-saved-query-btn');
          if (renameButton) {
              e.preventDefault();
              const form = renameButton.closest('form');
              const currentName = form?.querySelector('input[name="query_name"]')?.value || '';
              const nextName = window.prompt('Rename saved query', currentName);
              if (!nextName || nextName.trim() === '' || nextName.trim() === currentName) return;

              form.querySelector('input[name="new_query_name"]').value = nextName.trim();
              form.submit();
              return;
          }

          if (e.target.closest('.delete-query-folder-form')) {
              return;
          }

          const folderHeader = e.target.closest('.saved-query-folder[data-folder]');
          if (folderHeader) {
              const folderKey = normalizeFolderPath(folderHeader.dataset.folder || '');
              if (!folderKey) return;
              if (collapsedSqlFolders.has(folderKey)) {
                  collapsedSqlFolders.delete(folderKey);
              } else {
                  collapsedSqlFolders.add(folderKey);
              }
              saveCollapsedSqlFolders();
              filterSavedQueries();
              return;
          }

          const target = e.target.closest('a');
          if (target) { 
              e.preventDefault(); 
              const sql = target.getAttribute('data-sql'); 
              const name = target.getAttribute('data-name'); 
              const folder = target.getAttribute('data-folder') || '';
              editor.value = sql; 
              queryNameInput.value = name; 
              if (queryFolderInput) {
                  queryFolderInput.value = folder;
              }
              scanForVariables(); 
              handleInput(); 
          }
      });

      saveQueryForm.addEventListener('submit', (e) => {
          querySqlInput.value = editor.value;
          if (queryNameInput.value.trim() === '') { e.preventDefault(); }
      });

      function addVariable(name = '', value = '') {
          const div = document.createElement('div');
          div.className = 'var-input-group';
          const label = document.createElement('label');
          label.innerText = name || 'New Var';
          const input = document.createElement('input');
          input.type = 'text';
          input.name = name;
          input.value = value;
          input.placeholder = 'Value';
          
          if(!name) {
             input.placeholder = 'Name';
             input.onchange = (e) => { input.name = e.target.value; label.innerText = e.target.value; };
          }
          
          const closeBtn = document.createElement('span');
          closeBtn.className = 'var-del-btn';
          closeBtn.innerHTML = '&times;';
          closeBtn.title = 'Remove Variable';
          closeBtn.onclick = function() {
              div.remove();
          };

          div.appendChild(label);
          div.appendChild(input);
          div.appendChild(closeBtn); 
          
          const btn = variablesSection.querySelector('.add-var-btn');
          variablesSection.insertBefore(div, btn);
      }
      window.addVariable = addVariable;

      function scanForVariables() {
          const regex = /{{([^}]+)}}/g;
          const text = editor.value;
          let match;
          const foundVars = new Set();
          
          while ((match = regex.exec(text)) !== null) {
              foundVars.add(match[1]);
          }
          
          const currentInputs = Array.from(variablesSection.querySelectorAll('input'));
          const currentValues = {};
          currentInputs.forEach(i => { if(i.name) currentValues[i.name] = i.value; });
          
          const existingGroups = variablesSection.querySelectorAll('.var-input-group');
          existingGroups.forEach(g => g.remove());
          
          foundVars.forEach(v => {
              addVariable(v, currentValues[v] || '');
          });
      }
      
      const escapeHtml = (unsafe) => {
          return unsafe
               .replace(/&/g, "&amp;")
               .replace(/</g, "&lt;")
               .replace(/>/g, "&gt;")
               .replace(/"/g, "&quot;");
      };

      const applyHighlights = (text) => {
          let html = escapeHtml(text);
          
          const tokens = [];
          const pushToken = (text, type) => {
              const id = "___TOKEN" + tokens.length + "___";
              tokens.push({ id, text, type });
              return id;
          };
          
          html = html.replace(/(--.*$)/gm, (m) => pushToken(m, 'hl-comment'));
          
          html = html.replace(/('([^'\\]|\\.)*')/g, (m) => pushToken(m, 'hl-string'));
          
          const keywords = ["SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "JOIN", "INNER", "OUTER", "LEFT", "RIGHT", "ON", "GROUP", "BY", "ORDER", "LIMIT", "OFFSET", "AND", "OR", "NOT", "NULL", "AS", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN", "LIKE", "ILIKE", "IN", "IS", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END", "HAVING", "UNION", "ALL"];
          
          const rxKeyword = new RegExp(`\\b(${keywords.join('|')})\\b`, 'gi');
          html = html.replace(rxKeyword, '<span class="hl-keyword">$1</span>');
          
          html = html.replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>');
          
          tokens.forEach(t => {
              html = html.replace(t.id, `<span class="${t.type}">${t.text}</span>`);
          });
          
          if (text[text.length-1] === "\n") {
              html += " "; 
          }
          
          return html;
      };

      const handleInput = () => {
          const text = editor.value;
          highlights.innerHTML = applyHighlights(text);
          scanForVariables();
      };

      const syncScroll = () => {
          backdrop.scrollTop = editor.scrollTop;
          backdrop.scrollLeft = editor.scrollLeft;
      };

      editor.addEventListener('input', handleInput);
      editor.addEventListener('scroll', syncScroll);
      if (editor.value) handleInput();


      function getCaretCoordinates() {
        const div = document.createElement('div');
        const style = window.getComputedStyle(editor);
        for (const prop of style) {
          div.style[prop] = style.getPropertyValue(prop);
        }
        div.style.position = 'absolute';
        div.style.top = '0';
        div.style.left = '0';
        div.style.visibility = 'hidden';
        div.style.height = 'auto';
        div.style.width = editor.offsetWidth + 'px';
        div.style.overflow = 'hidden';
        div.style.whiteSpace = 'pre-wrap';

        const text = editor.value.substring(0, editor.selectionStart);
        div.textContent = text;
        const span = document.createElement('span');
        span.textContent = '.';
        div.appendChild(span);
        
        document.body.appendChild(div);
        
        const coordinates = {
          top: span.offsetTop + parseInt(style.borderTopWidth) + parseInt(style.paddingTop) - editor.scrollTop,
          left: span.offsetLeft + parseInt(style.borderLeftWidth) + parseInt(style.paddingLeft) - editor.scrollLeft,
          lineHeight: parseInt(style.lineHeight) || 20 
        };
        document.body.removeChild(div);
        return coordinates;
      }

      function getAliases(sql) {
        const aliases = {};
        // Match: FROM table alias OR JOIN table alias
        // Also supports: table AS alias
        const regex = /\b(?:FROM|JOIN)\s+([a-zA-Z0-9_]+)(?:\s+AS)?\s+([a-zA-Z0-9_]+)\b/gi;
        let match;
        while ((match = regex.exec(sql)) !== null) {
            const table = match[1];
            const alias = match[2];
            // Don't treat common SQL keywords as aliases if they appear after a table
            const keywords = ["WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "ON", "GROUP", "BY", "ORDER", "LIMIT", "OFFSET", "AND", "OR"];
            if (!keywords.includes(alias.toUpperCase())) {
                aliases[alias.toLowerCase()] = table;
            }
        }
        return aliases;
      }

      editor.addEventListener('input', function(e) {
          const val = this.value;
          const cursorPosition = this.selectionStart;
          const textBeforeCursor = val.substring(0, cursorPosition);
          
          const words = textBeforeCursor.split(/[\s,()]+/);
          const currentWord = words[words.length - 1];
          
          if (!currentWord) {
              closeAutocomplete();
              return;
          }

          let matches = [];
          
          if (currentWord.includes('.')) {
              const parts = currentWord.split('.');
              const prefix = parts[0];
              const colPrefix = parts[1] || '';
              
              // 1. Try to find a direct table match
              let targetTable = Object.keys(dbSchema).find(t => t.toUpperCase() === prefix.toUpperCase());
              
              // 2. If no direct match, try to resolve as an alias
              if (!targetTable) {
                  const aliases = getAliases(val);
                  const aliasedTable = aliases[prefix.toLowerCase()];
                  if (aliasedTable) {
                      targetTable = Object.keys(dbSchema).find(t => t.toUpperCase() === aliasedTable.toUpperCase());
                  }
              }
              
              if (targetTable && dbSchema[targetTable]) {
                  matches = dbSchema[targetTable]
                      .filter(col => col.toUpperCase().startsWith(colPrefix.toUpperCase()))
                      .map(col => ({ display: col, insert: col, type: 'column' }));
              }
          } 
          else {
              // Standard table suggestions
              matches = Object.keys(dbSchema)
                  .filter(t => t.toUpperCase().startsWith(currentWord.toUpperCase()))
                  .map(t => ({ display: t, insert: t, type: 'table' }));

              // Keyword suggestions (optional but helpful)
              const keywords = ["SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "JOIN", "ORDER BY", "GROUP BY", "LIMIT", "CREATE TABLE", "DROP TABLE"];
              const kwMatches = keywords
                  .filter(k => k.startsWith(currentWord.toUpperCase()) && currentWord.length >= 2)
                  .map(k => ({ display: k, insert: k, type: 'keyword' }));
              matches = [...matches, ...kwMatches];
          }

          if (matches.length > 0) {
              currentFocus = -1;
              showAutocomplete(matches, currentWord);
          } else {
              closeAutocomplete();
          }
      });

      function showAutocomplete(matches, currentWord) {
          autocompleteList.innerHTML = "";
          const coords = getCaretCoordinates();
          const rect = editor.getBoundingClientRect();
          
          autocompleteList.style.display = "block";
          autocompleteList.style.left = (rect.left + coords.left + window.scrollX) + "px";
          autocompleteList.style.top = (rect.top + coords.top + coords.lineHeight + window.scrollY) + "px";
          
          matches.forEach(match => {
              const div = document.createElement("div");
              div.innerHTML = `<strong>${match.display.substr(0, currentWord.length)}</strong>${match.display.substr(currentWord.length)} <small style='float:right; opacity:0.6;'>${match.type}</small>`;
              div.addEventListener("click", function(e) {
                  insertAtCursor(editor, match.insert, currentWord);
                  closeAutocomplete();
              });
              autocompleteList.appendChild(div);
          });
      }

      function closeAutocomplete() {
          autocompleteList.innerHTML = "";
          autocompleteList.style.display = "none";
      }
      
      document.addEventListener("click", function (e) {
          if (e.target !== editor) { closeAutocomplete(); }
      });

      editor.addEventListener('keydown', function(e) {
          const list = document.getElementById('autocomplete-list');
          if (!list || list.style.display === 'none') return;
          
          const items = list.getElementsByTagName('div');
          
          if (e.key === 'ArrowDown') {
              currentFocus++;
              addActive(items);
              e.preventDefault(); 
          } else if (e.key === 'ArrowUp') {
              currentFocus--;
              addActive(items);
              e.preventDefault(); 
          } else if (e.key === 'Enter') {
              e.preventDefault(); 
              if (currentFocus > -1) {
                  if (items[currentFocus]) items[currentFocus].click();
              }
          } else if (e.key === 'Escape') {
              closeAutocomplete();
          }
      });

      function addActive(items) {
          if (!items) return;
          removeActive(items);
          if (currentFocus >= items.length) currentFocus = 0;
          if (currentFocus < 0) currentFocus = (items.length - 1);
          items[currentFocus].classList.add('autocomplete-active');
          items[currentFocus].scrollIntoView({block: 'nearest'});
      }

      function removeActive(items) {
          for (let i = 0; i < items.length; i++) {
              items[i].classList.remove('autocomplete-active');
          }
      }

      function insertAtCursor(field, value, typedWord) {
          let prefix = "";
          if (typedWord.includes('.')) {
             prefix = typedWord.split('.')[0] + '.';
          }
          
          const valToInsert = value; 
          
          const cursorPos = field.selectionStart;
          const textBefore = field.value.substring(0, cursorPos);
          const textAfter = field.value.substring(cursorPos);
          
          const cleanBefore = textBefore.substring(0, textBefore.length - (typedWord.length - prefix.length));
          
          field.value = cleanBefore + valToInsert + textAfter;
          field.selectionStart = field.selectionEnd = cleanBefore.length + valToInsert.length;
          field.focus();
          
          handleInput();
      }

      if (editor.value === "") { editor.value = "SELECT 1;"; handleInput(); }
