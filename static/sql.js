const schemaTemplate = document.getElementById('sql-schema-data');
let dbSchema = schemaTemplate ? JSON.parse(schemaTemplate.textContent || '{}') : {};
const functionsTemplate = document.getElementById('sql-functions-data');
let dbFunctions = functionsTemplate ? JSON.parse(functionsTemplate.textContent || '[]') : [];

const mainContent = document.getElementById('main');
      const editor = document.getElementById('sql-editor');
      const sidebarSearchInput = document.getElementById('sidebar-search-input');
      const sidebarTableList = document.getElementById('table-list');
      const sidebarFunctionList = document.getElementById('function-list');
      const schemaTabTables = document.getElementById('schema-tab-tables');
      const schemaTabFunctions = document.getElementById('schema-tab-functions');
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
      const variablesLeft = variablesSection?.querySelector('.variables-left') || variablesSection;
      const autocompleteList = document.getElementById('autocomplete-list');
      const saveSqlFileBtn = document.getElementById('save-sql-file-btn');
      let currentFocus = -1;
      
      const backdrop = document.getElementById('sql-backdrop');
      const highlights = backdrop.querySelector('.highlights');
      const connectionNickname = document.querySelector("input[name=connection]")?.value || "";
      const connectionDbType = document.getElementById('sql-active-connection')?.dataset.dbType || '';
      const activeSqlTabId = new URLSearchParams(window.location.search).get('tab') || 'default';
      const sqlWorkspaceStorageKey = `sql_workspace_${connectionNickname}_${activeSqlTabId}`;
      const varsStorageKey = "sql_vars_" + connectionNickname;
      const savedQueryFoldersCollapsedKey = "sql_saved_query_folders_collapsed_" + connectionNickname;
      let collapsedSqlFolders = readCollapsedSqlFolders();
      let isRestoringWorkspace = false;

      function tauriInvoke(command, payload = {}) {
          if (window.__TAURI__?.core?.invoke) {
              return window.__TAURI__.core.invoke(command, payload);
          }

          if (window.OGDEVDESK_DESKTOP_MODE === true && window.parent !== window) {
              return new Promise((resolve, reject) => {
                  const id = `sql-save-${Date.now()}-${Math.random().toString(36).slice(2)}`;
                  const timeout = window.setTimeout(() => {
                      window.removeEventListener('message', onMessage);
                      reject(new Error('Desktop save dialog did not respond.'));
                  }, 120000);

                  function onMessage(event) {
                      if (event.source !== window.parent) return;
                      const data = event.data || {};
                      if (data.type !== 'ogdevdesk-tauri-result' || data.id !== id) return;
                      window.clearTimeout(timeout);
                      window.removeEventListener('message', onMessage);
                      if (data.ok) {
                          resolve(data.result);
                      } else {
                          reject(new Error(data.error || 'Desktop command failed.'));
                      }
                  }

                  window.addEventListener('message', onMessage);
                  window.parent.postMessage({
                      type: 'ogdevdesk-tauri-invoke',
                      id,
                      command,
                      payload,
                  }, '*');
              });
          }

          return null;
      }

      function browserDownloadFile(filename, content, type) {
          const blob = new Blob([content], { type });
          const link = document.createElement('a');
          const url = URL.createObjectURL(blob);
          link.href = url;
          link.download = filename;
          link.style.visibility = 'hidden';
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
          URL.revokeObjectURL(url);
      }

      async function saveGeneratedFile(filename, content, type) {
          const savePromise = tauriInvoke('save_text_file', {
              suggestedFilename: filename,
              contents: content,
          });
          if (savePromise) {
              await savePromise;
              return;
          }

          if (window.showSaveFilePicker) {
              const extension = filename.split('.').pop() || '';
              const handle = await window.showSaveFilePicker({
                  suggestedName: filename,
                  types: extension ? [{
                      description: `${extension.toUpperCase()} file`,
                      accept: { [type]: [`.${extension}`] },
                  }] : undefined,
              });
              const writable = await handle.createWritable();
              await writable.write(new Blob([content], { type }));
              await writable.close();
              return;
          }

          browserDownloadFile(filename, content, type);
      }

      function currentSqlEditorText() {
          const candidates = [
              editor?.value || '',
              document.querySelector('textarea[name="sql"]')?.value || '',
              document.getElementById('query-sql')?.value || '',
          ];

          try {
              const saved = JSON.parse(localStorage.getItem(sqlWorkspaceStorageKey) || '{}');
              if (saved && typeof saved.sql === 'string') {
                  candidates.push(saved.sql);
              }
          } catch (_) {}

          return candidates.reduce((longest, value) => (
              String(value || '').length > String(longest || '').length ? value : longest
          ), '');
      }

      window.getSqlAssistantContext = function(flags = {}) {
          const includeTables = flags.includeSqlTables !== false;
          const includeFunctions = flags.includeSqlFunctions !== false;
          const includeEditor = flags.includeEditor !== false;
          const includePage = flags.includePage !== false;
          const includeSqlOutput = Boolean(flags.includeSqlOutput || flags.includeResponse);
          const tableNames = Object.keys(dbSchema || {});
          const functionNames = Array.isArray(dbFunctions)
              ? dbFunctions.map((fn) => fn.name || fn.signature || '').filter(Boolean)
              : [];
          const outputTable = getActiveOutputTable();
          const outputHeaders = outputTable
              ? Array.from(outputTable.querySelectorAll('thead th')).map((th) => th.innerText.trim()).filter(Boolean)
              : [];

          const context = {
              page: 'sql',
              connection: connectionNickname,
              tabId: activeSqlTabId,
          };

          if (includePage) {
              context.queryName = queryNameInput?.value || '';
              context.queryFolder = queryFolderInput?.value || '';
              context.rowCount = document.getElementById('row-count')?.innerText || '';
              context.outputColumns = outputHeaders;
          }

          if (includeEditor) {
              const sqlEditor = currentSqlEditorText();
              context.sqlEditor = sqlEditor;
              context.sqlEditorLength = sqlEditor.length;
              context.sqlEditorSource = sqlEditor === (editor?.value || '') ? 'textarea' : 'workspace fallback';
          }

          if (includeTables) {
              context.databaseSchema = {
                  tables: tableNames.map((name) => ({
                      name,
                      columns: Array.isArray(dbSchema[name]) ? dbSchema[name] : [],
                  })),
              };
          }

          if (includeFunctions) {
              context.databaseFunctions = Array.isArray(dbFunctions)
                  ? dbFunctions.map((fn) => ({
                      name: fn.name || '',
                      signature: fn.signature || '',
                      definition: fn.definition || '',
                  }))
                  : functionNames.map((name) => ({ name }));
          }

          if (includeSqlOutput) {
              context.sqlOutput = (output?.innerText || '').slice(0, 12000);
          }

          return context;
      };

      function resetSqlOutputPane() {
          output.innerHTML = "<pre>Click a table name or enter a query and press 'Run Query'.</pre>";
          if (outputFilterInput) {
              outputFilterInput.value = '';
          }
          const countSpan = document.getElementById('row-count');
          if (countSpan) {
              countSpan.innerText = '';
          }
          if (outputHistorySelect) {
              outputHistorySelect.value = '';
          }
          if (columnMenuPanel) {
              columnMenuPanel.innerHTML = '<div class="sql-result-menu-empty">Run a query to choose columns.</div>';
              columnMenuPanel.hidden = true;
          }
          if (columnMenuBtn) {
              columnMenuBtn.setAttribute('aria-expanded', 'false');
          }
          updateSelectionCount();
      }

      function openUntitledSqlFile() {
          editor.value = '';
          queryNameInput.value = '';
          if (queryFolderInput) {
              queryFolderInput.value = '';
          }
          resetSqlOutputPane();
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
              openUntitledSqlFile();
          });
      }

      saveSqlFileBtn.addEventListener('click', async () => {
          const content = editor.value;
          if (!content) return;

          let filename = queryNameInput.value.trim() || 'query';
          if (!filename.toLowerCase().endsWith('.sql')) filename += '.sql';

          try {
              await saveGeneratedFile(filename, content, 'text/plain');
          } catch (error) {
              if (error?.name === 'AbortError') return;
              alert(`Failed to save SQL file: ${error.message || error}`);
          }
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
      const schemaListPanes = [sidebarTableList, sidebarFunctionList].filter(Boolean);
      let isTableListResizing = false;
      let lastTableListDownY = 0;
      let startTableListHeight = 0;

      function activeSchemaListPane() {
          return activeSchemaTab() === 'functions' ? sidebarFunctionList : sidebarTableList;
      }

      function applySchemaListHeight(height) {
          schemaListPanes.forEach((pane) => {
              pane.style.flex = '0 0 auto';
              pane.style.height = `${height}px`;
          });
      }

      const savedTableListHeight = Number(localStorage.getItem(tableListHeightKey));
      if (savedTableListHeight > 40) {
          applySchemaListHeight(savedTableListHeight);
      }

      if (tableQueryResizer && tableListPane) {
          tableQueryResizer.addEventListener('mousedown', (event) => {
              isTableListResizing = true;
              lastTableListDownY = event.clientY;
              startTableListHeight = activeSchemaListPane()?.offsetHeight || tableListPane.offsetHeight;
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

          applySchemaListHeight(nextHeight);
      });

      document.addEventListener('mouseup', () => {
          if (!isTableListResizing) return;

          isTableListResizing = false;
          tableQueryResizer.classList.remove('resizing');
          document.body.style.cursor = '';
          document.body.style.userSelect = '';
          localStorage.setItem(tableListHeightKey, (activeSchemaListPane()?.offsetHeight || tableListPane.offsetHeight).toString());
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
                  a.title = "Click to browse live table";
                  a.onclick = (e) => { e.preventDefault(); openLiveTableBrowser(tableName); };
                  li.appendChild(a);
                  sidebarTableList.appendChild(li);
              }
          });
      }

      function functionDisplayText(fn) {
          const header = [
              `-- Function: ${fn.signature || fn.name || ''}`,
              fn.return_type ? `-- Returns: ${fn.return_type}` : '',
          ].filter(Boolean).join('\n');
          return `${header}\n\n${fn.definition || ''}`.trim();
      }

      function openFunctionDisplay(fn) {
          editor.value = functionDisplayText(fn);
          queryNameInput.value = '';
          if (queryFolderInput) {
              queryFolderInput.value = '';
          }
          handleInput();
          scanForVariables();
          output.innerHTML = `
              <div class="sql-function-display">
                  <div><strong>Function</strong>: ${escapeHtmlText(fn.signature || fn.name || '')}</div>
                  ${fn.return_type ? `<div><strong>Returns</strong>: ${escapeHtmlText(fn.return_type)}</div>` : ''}
                  <div class="sql-function-display-note">Definition loaded into the SQL editor for inspection.</div>
              </div>
          `;
      }

      function renderFunctionList() {
          const filter = sidebarSearchInput.value.toUpperCase();
          sidebarFunctionList.innerHTML = '';

          if (!Array.isArray(dbFunctions) || dbFunctions.length === 0) {
              const empty = document.createElement('li');
              empty.className = 'schema-list-empty';
              empty.textContent = 'No functions found';
              sidebarFunctionList.appendChild(empty);
              return;
          }

          dbFunctions
              .slice()
              .sort((a, b) => String(a.signature || a.name || '').localeCompare(String(b.signature || b.name || '')))
              .forEach((fn) => {
                  const label = fn.signature || fn.name || '';
                  if (label.toUpperCase().indexOf(filter) === -1) return;

                  const li = document.createElement('li');
                  const a = document.createElement('a');
                  a.className = 'function-list-item';
                  a.textContent = label;
                  a.href = '#';
                  a.title = 'Click to inspect function definition';
                  a.addEventListener('click', (e) => {
                      e.preventDefault();
                      openFunctionDisplay(fn);
                  });
                  li.appendChild(a);
                  sidebarFunctionList.appendChild(li);
              });

          if (!sidebarFunctionList.children.length) {
              const empty = document.createElement('li');
              empty.className = 'schema-list-empty';
              empty.textContent = 'No matching functions';
              sidebarFunctionList.appendChild(empty);
          }
      }

      function activeSchemaTab() {
          return schemaTabFunctions?.classList.contains('active') ? 'functions' : 'tables';
      }

      function renderActiveSchemaList() {
          if (activeSchemaTab() === 'functions') {
              renderFunctionList();
          } else {
              renderTableList();
          }
      }

      function setSchemaTab(tabName) {
          const showFunctions = tabName === 'functions';
          schemaTabTables?.classList.toggle('active', !showFunctions);
          schemaTabFunctions?.classList.toggle('active', showFunctions);
          schemaTabTables?.setAttribute('aria-selected', showFunctions ? 'false' : 'true');
          schemaTabFunctions?.setAttribute('aria-selected', showFunctions ? 'true' : 'false');
          sidebarTableList.hidden = showFunctions;
          sidebarFunctionList.hidden = !showFunctions;
          sidebarSearchInput.placeholder = showFunctions ? 'Search functions...' : 'Search tables...';
          sidebarSearchInput.value = '';
          renderActiveSchemaList();
      }

      renderTableList();
      renderFunctionList();
      schemaTabTables?.addEventListener('click', () => setSchemaTab('tables'));
      schemaTabFunctions?.addEventListener('click', () => setSchemaTab('functions'));
      sidebarSearchInput.addEventListener('keyup', renderActiveSchemaList);

      function tableFilterStorageKey(tableName) {
          return `sql_table_filters_${connectionNickname}_${tableName}`;
      }

      function escapeHtmlText(value) {
          return String(value || '')
              .replace(/&/g, '&amp;')
              .replace(/</g, '&lt;')
              .replace(/>/g, '&gt;')
              .replace(/"/g, '&quot;');
      }

      function escapeAttribute(value) {
          return escapeHtmlText(value).replace(/'/g, '&#39;');
      }

      function loadSavedTableFilters(tableName) {
          try {
              const filters = JSON.parse(localStorage.getItem(tableFilterStorageKey(tableName)) || '[]');
              return Array.isArray(filters) ? filters : [];
          } catch (_) {
              return [];
          }
      }

      function saveSavedTableFilters(tableName, filters) {
          localStorage.setItem(tableFilterStorageKey(tableName), JSON.stringify(filters));
      }

      function quoteSqlIdentifierForDisplay(identifier) {
          return `"${String(identifier || '').replace(/"/g, '""')}"`;
      }

      function quoteSqlLiteralForDisplay(value) {
          return `'${String(value || '').replace(/'/g, "''")}'`;
      }

      function tableReferenceForDisplay(tableName) {
          if (connectionDbType === 'postgres') {
              return `"public".${quoteSqlIdentifierForDisplay(tableName)}`;
          }
          return quoteSqlIdentifierForDisplay(tableName);
      }

      function tableFilterSqlForDisplay(columnExpression, op, value) {
          const valueLiteral = quoteSqlLiteralForDisplay(value);
          switch (op) {
              case 'is_null':
                  return `${columnExpression} IS NULL`;
              case 'not_null':
                  return `${columnExpression} IS NOT NULL`;
              case 'eq':
                  return `${columnExpression} = ${valueLiteral}`;
              case 'not_eq':
                  return `${columnExpression} <> ${valueLiteral}`;
              case 'contains':
                  return `CAST(${columnExpression} AS TEXT) LIKE ${quoteSqlLiteralForDisplay(`%${value}%`)}`;
              case 'begins_with':
                  return `CAST(${columnExpression} AS TEXT) LIKE ${quoteSqlLiteralForDisplay(`${value}%`)}`;
              case 'ends_with':
                  return `CAST(${columnExpression} AS TEXT) LIKE ${quoteSqlLiteralForDisplay(`%${value}`)}`;
              case 'like':
                  return `CAST(${columnExpression} AS TEXT) LIKE ${valueLiteral}`;
              default:
                  return '';
          }
      }

      function tableWhereSqlForDisplay(filters, columns) {
          const clauses = [];
          filters.forEach((filter) => {
              const op = filter.op || 'contains';
              const value = filter.value || '';
              const isNullOp = op === 'is_null' || op === 'not_null';
              if (!isNullOp && value === '') return;

              if (!filter.column) {
                  const anyClauses = columns
                      .map((column) => tableFilterSqlForDisplay(quoteSqlIdentifierForDisplay(column), op, value))
                      .filter(Boolean);
                  if (anyClauses.length > 0) {
                      clauses.push(`(${anyClauses.join(' OR ')})`);
                  }
                  return;
              }

              const clause = tableFilterSqlForDisplay(quoteSqlIdentifierForDisplay(filter.column), op, value);
              if (clause) clauses.push(clause);
          });

          return clauses.length > 0 ? `\nWHERE ${clauses.join('\n  AND ')}` : '';
      }

      function tableBrowserSqlForDisplay(tableName, columns, filters, page, pageSize) {
          const offset = Math.max(0, (page - 1) * pageSize);
          return [
              `SELECT *`,
              `FROM ${tableReferenceForDisplay(tableName)}${tableWhereSqlForDisplay(filters, columns)}`,
              `LIMIT ${pageSize + 1}`,
              `OFFSET ${offset};`
          ].join('\n');
      }

      function rowUpdateSqlForDisplay(tableName, columns, original, current) {
          const setClauses = [];
          const whereClauses = [];
          columns.forEach((column) => {
              const originalValue = original[column] || '';
              const currentValue = current[column] || '';
              const columnSql = quoteSqlIdentifierForDisplay(column);
              whereClauses.push(`${columnSql} = ${quoteSqlLiteralForDisplay(originalValue)}`);
              if (originalValue !== currentValue) {
                  setClauses.push(`${columnSql} = ${quoteSqlLiteralForDisplay(currentValue)}`);
              }
          });

          if (setClauses.length === 0) return '';
          return `UPDATE ${tableReferenceForDisplay(tableName)}\nSET ${setClauses.join(',\n    ')}\nWHERE ${whereClauses.join('\n  AND ')};`;
      }

      function tableBrowserUpdateSqlForDisplay(tableName, columns, changes) {
          return changes
              .map((change) => rowUpdateSqlForDisplay(tableName, columns, change.original, change.current))
              .filter(Boolean)
              .join('\n\n');
      }

      function tableBrowserFilterRow(columns, filter = {}) {
          const row = document.createElement('div');
          row.className = 'table-browser-filter-row';
          row.innerHTML = `
              <select class="table-filter-column" title="Column">
                  <option value="">Any column</option>
                  ${columns.map((column) => `<option value="${escapeAttribute(column)}">${escapeHtmlText(column)}</option>`).join('')}
              </select>
              <select class="table-filter-op" title="Filter operation">
                  <option value="contains">contains</option>
                  <option value="eq">eq</option>
                  <option value="not_eq">not eq</option>
                  <option value="is_null">is null</option>
                  <option value="not_null">not null</option>
                  <option value="begins_with">begins with</option>
                  <option value="ends_with">ends with</option>
                  <option value="like">like</option>
              </select>
              <input class="table-filter-value" type="text" placeholder="Value">
              <button type="button" class="add-var-btn table-filter-remove">x</button>
          `;
          row.querySelector('.table-filter-column').value = filter.column || '';
          row.querySelector('.table-filter-op').value = filter.op || 'contains';
          row.querySelector('.table-filter-value').value = filter.value || '';
          row.querySelector('.table-filter-remove').addEventListener('click', () => row.remove());
          return row;
      }

      function readTableBrowserFilters(container) {
          return Array.from(container.querySelectorAll('.table-browser-filter-row')).map((row) => ({
              column: row.querySelector('.table-filter-column')?.value || '',
              op: row.querySelector('.table-filter-op')?.value || 'contains',
              value: row.querySelector('.table-filter-value')?.value || '',
          }));
      }

      function renderSavedTableFilterOptions(select, tableName) {
          select.innerHTML = '<option value="">Saved filters</option>';
          loadSavedTableFilters(tableName).forEach((preset) => {
              const option = document.createElement('option');
              option.value = preset.name;
              option.textContent = preset.name;
              select.appendChild(option);
          });
      }

      function openLiveTableBrowser(tableName) {
          const columns = dbSchema[tableName] || [];
          let page = 1;
          const pageSize = 100;
          output.innerHTML = `
              <div class="table-browser" data-table="${escapeAttribute(tableName)}">
                  <div class="table-browser-toolbar">
                      <strong>${escapeHtmlText(tableName)}</strong>
                      <button type="button" class="add-var-btn table-browser-add-filter">+ Filter</button>
                      <button type="button" class="add-var-btn table-browser-apply">Apply</button>
                      <button type="button" class="add-var-btn table-browser-save-filter">Save Filter</button>
                      <select class="table-browser-saved-filters"></select>
                      <button type="button" class="add-var-btn table-browser-prev">Prev</button>
                      <span class="table-browser-page">Page 1</span>
                      <button type="button" class="add-var-btn table-browser-next">Next</button>
                      <div class="table-browser-change-actions" hidden>
                          <button type="button" class="add-var-btn table-browser-discard-changes">Discard Changes</button>
                          <button type="button" class="add-var-btn table-browser-save-changes">Save Changes</button>
                      </div>
                  </div>
                  <div class="table-browser-filters"></div>
                  <div class="table-browser-status">Loading...</div>
                  <div class="table-browser-results"></div>
              </div>
          `;

          const browser = output.querySelector('.table-browser');
          const filtersContainer = browser.querySelector('.table-browser-filters');
          const savedSelect = browser.querySelector('.table-browser-saved-filters');
          const status = browser.querySelector('.table-browser-status');
          const results = browser.querySelector('.table-browser-results');
          const pageLabel = browser.querySelector('.table-browser-page');
          const prevBtn = browser.querySelector('.table-browser-prev');
          const nextBtn = browser.querySelector('.table-browser-next');
          const changeActions = browser.querySelector('.table-browser-change-actions');
          const discardChangesBtn = browser.querySelector('.table-browser-discard-changes');
          const saveChangesBtn = browser.querySelector('.table-browser-save-changes');
          const countSpan = document.getElementById('row-count');
          let baseBrowserSql = '';
          let pendingChanges = [];
          if (countSpan) countSpan.innerText = '';
          renderSavedTableFilterOptions(savedSelect, tableName);

          function collectPendingTableChanges() {
              const table = results.querySelector('table');
              if (!table) return [];

              return Array.from(table.querySelectorAll('tbody tr'))
                  .map((row) => {
                      const original = {};
                      const current = {};
                      let changed = false;

                      columns.forEach((column, index) => {
                          const cell = row.children[index];
                          const originalValue = cell?.dataset.originalValue || '';
                          const currentValue = cell?.innerText || '';
                          original[column] = originalValue;
                          current[column] = currentValue;
                          if (originalValue !== currentValue) {
                              changed = true;
                          }
                      });

                      return changed ? { original, current } : null;
                  })
                  .filter(Boolean);
          }

          function updateTableBrowserEditorSql() {
              pendingChanges = collectPendingTableChanges();
              const updateSql = tableBrowserUpdateSqlForDisplay(tableName, columns, pendingChanges);
              editor.value = updateSql
                  ? `${baseBrowserSql}\n\n-- Pending table edit SQL\n${updateSql}`
                  : baseBrowserSql;
              if (changeActions) changeActions.hidden = pendingChanges.length === 0;
              scanForVariables();
              handleInput();
          }

          function makeTableBrowserEditable(table) {
              Array.from(table.querySelectorAll('tbody tr')).forEach((row) => {
                  Array.from(row.children).forEach((cell) => {
                      cell.dataset.originalValue = cell.innerText || '';
                      cell.contentEditable = 'true';
                      cell.spellcheck = false;
                      cell.classList.add('table-browser-editable-cell');
                      cell.addEventListener('input', () => {
                          cell.classList.toggle('table-browser-cell-edited', (cell.innerText || '') !== (cell.dataset.originalValue || ''));
                          updateTableBrowserEditorSql();
                      });
                      cell.addEventListener('keydown', (event) => {
                          if (event.key === 'Enter') {
                              event.preventDefault();
                              cell.blur();
                          }
                      });
                  });
              });
          }

          const fetchPage = async () => {
              status.textContent = 'Loading...';
              const activeFilters = readTableBrowserFilters(filtersContainer);
              baseBrowserSql = tableBrowserSqlForDisplay(tableName, columns, activeFilters, page, pageSize);
              editor.value = baseBrowserSql;
              queryNameInput.value = '';
              if (queryFolderInput) queryFolderInput.value = '';
              scanForVariables();
              handleInput();
              const resp = await fetch('/sql/table-data', {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({
                      connection: connectionNickname,
                      table: tableName,
                      page,
                      page_size: pageSize,
                      filters: activeFilters,
                  }),
              });
              if (!resp.ok) throw new Error(await resp.text());
              const data = await resp.json();
              results.innerHTML = data.html;
              const table = results.querySelector('table');
              if (table) {
                  makeTableInteractable(table);
                  makeTableBrowserEditable(table);
              }
              status.textContent = data.row_count_text || '0 rows';
              if (countSpan) countSpan.innerText = data.row_count_text || '0 rows';
              pageLabel.textContent = `Page ${data.page}`;
              prevBtn.disabled = data.page <= 1;
              nextBtn.disabled = !data.has_next;
              pendingChanges = [];
              if (changeActions) changeActions.hidden = true;
          };

          const safeFetchPage = () => {
              fetchPage().catch((error) => {
                  status.textContent = `Error: ${error.message}`;
              });
          };

          browser.querySelector('.table-browser-add-filter').addEventListener('click', () => {
              filtersContainer.appendChild(tableBrowserFilterRow(columns));
          });
          browser.querySelector('.table-browser-apply').addEventListener('click', () => {
              page = 1;
              safeFetchPage();
          });
          browser.querySelector('.table-browser-save-filter').addEventListener('click', () => {
              const name = window.prompt('Save table filter as');
              if (!name || !name.trim()) return;
              const filters = loadSavedTableFilters(tableName).filter((preset) => preset.name !== name.trim());
              filters.push({ name: name.trim(), filters: readTableBrowserFilters(filtersContainer) });
              saveSavedTableFilters(tableName, filters);
              renderSavedTableFilterOptions(savedSelect, tableName);
              savedSelect.value = name.trim();
          });
          savedSelect.addEventListener('change', () => {
              const preset = loadSavedTableFilters(tableName).find((item) => item.name === savedSelect.value);
              filtersContainer.innerHTML = '';
              if (preset) {
                  preset.filters.forEach((filter) => filtersContainer.appendChild(tableBrowserFilterRow(columns, filter)));
                  page = 1;
                  safeFetchPage();
              }
          });
          prevBtn.addEventListener('click', () => {
              page = Math.max(1, page - 1);
              safeFetchPage();
          });
          nextBtn.addEventListener('click', () => {
              page += 1;
              safeFetchPage();
          });
          discardChangesBtn.addEventListener('click', () => {
              const table = results.querySelector('table');
              if (!table) return;
              table.querySelectorAll('tbody td').forEach((cell) => {
                  cell.innerText = cell.dataset.originalValue || '';
                  cell.classList.remove('table-browser-cell-edited');
              });
              updateTableBrowserEditorSql();
          });
          saveChangesBtn.addEventListener('click', async () => {
              pendingChanges = collectPendingTableChanges();
              if (pendingChanges.length === 0) return;
              if (!window.confirm(`Save ${pendingChanges.length} edited row(s) to ${tableName}?`)) return;

              saveChangesBtn.disabled = true;
              status.textContent = 'Saving table edits...';
              try {
                  const resp = await fetch('/sql/table-update', {
                      method: 'POST',
                      headers: { 'Content-Type': 'application/json' },
                      body: JSON.stringify({
                          connection: connectionNickname,
                          tab_id: activeSqlTabId,
                          table: tableName,
                          changes: pendingChanges,
                      }),
                  });
                  if (!resp.ok) throw new Error(await resp.text());
                  const result = await resp.json();
                  await refreshOutputHistoryOptions();
                  alert(result.message || 'Table edit complete.');
                  if (result.status === 'completed') {
                      safeFetchPage();
                  } else {
                      status.textContent = result.message || 'Table edit did not apply.';
                  }
              } catch (error) {
                  alert(`Table edit failed: ${error.message}`);
                  status.textContent = `Save failed: ${error.message}`;
              } finally {
                  saveChangesBtn.disabled = false;
              }
          });

          safeFetchPage();
      }

      const refreshBtn = document.getElementById('refresh-schema-btn');
      refreshBtn.addEventListener('click', refreshSchema);

      async function refreshSchema() {
          refreshBtn.style.animation = "spin 1s linear infinite";
          try {
              const [schemaResp, functionsResp] = await Promise.all([
                  fetch('/sql/' + connectionNickname + '/schema-json'),
                  fetch('/sql/' + connectionNickname + '/functions-json'),
              ]);
              if (schemaResp.ok) {
                  dbSchema = await schemaResp.json();
              } else {
                  console.error("Failed to refresh schema");
              }
              if (functionsResp.ok) {
                  dbFunctions = await functionsResp.json();
              } else {
                  console.error("Failed to refresh functions");
              }
              renderActiveSchemaList();
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
      const clearOutputBtn = document.getElementById('clear-output-btn');
      const maxOutputHistoryEntries = 8;
      const maxOutputHistoryEntryChars = 500000;
      const maxOutputHistoryTotalChars = 1500000;
      let outputHistoryCache = [];
      let activeSqlJobId = '';
      let sqlJobPollTimer = null;

      function getTableBodyRows(table) {
          if (!table) return [];
          const tbodyRows = Array.from(table.querySelectorAll('tbody tr'));
          if (tbodyRows.length > 0) return tbodyRows;
          return Array.from(table.querySelectorAll('tr')).filter((row) => !row.closest('thead'));
      }

      function getActiveOutputTable() {
          const tables = Array.from(output?.querySelectorAll('table') || [])
              .filter((table) => !table.closest('.sql-sticky-header-clone'));
          return tables.find((table) => getTableBodyRows(table).length > 0) || tables[0] || null;
      }

      function currentRowCountText() {
          return document.getElementById('row-count')?.innerText || '';
      }

      function serializableOutputHtml() {
          const clone = output.cloneNode(true);
          clone.querySelectorAll('.sql-sticky-header-clone, colgroup, .column-sort-indicator, .column-resize-handle').forEach((element) => {
              element.remove();
          });
          clone.querySelectorAll('.selected-row').forEach((row) => {
              row.classList.remove('selected-row');
          });
          return clone.innerHTML;
      }

      function saveSqlWorkspaceState() {
          if (isRestoringWorkspace || !connectionNickname) return;

          try {
              localStorage.setItem(sqlWorkspaceStorageKey, JSON.stringify({
                  sql: editor.value,
                  queryName: queryNameInput.value,
                  queryFolder: queryFolderInput ? queryFolderInput.value : '',
                  outputHtml: serializableOutputHtml(),
                  rowCountText: currentRowCountText(),
                  outputHistoryId: outputHistorySelect ? outputHistorySelect.value : '',
                  updatedAt: new Date().toISOString(),
              }));
              if (typeof window.renderSqlConnectionTabs === 'function') {
                  window.renderSqlConnectionTabs();
              }
          } catch (error) {
              console.error('Failed to save SQL tab workspace', error);
          }
      }

      function restoreSqlWorkspaceState() {
          try {
              const raw = localStorage.getItem(sqlWorkspaceStorageKey);
              if (!raw) return false;

              const state = JSON.parse(raw);
              isRestoringWorkspace = true;
              editor.value = state.sql || '';
              queryNameInput.value = state.queryName || '';
              if (queryFolderInput) {
                  queryFolderInput.value = state.queryFolder || '';
              }
              if (state.outputHtml) {
                  applyOutputHtml(state.outputHtml, state.rowCountText || '');
              }
              if (outputHistorySelect && state.outputHistoryId) {
                  outputHistorySelect.value = state.outputHistoryId;
              }
              return true;
          } catch (error) {
              console.error('Failed to restore SQL tab workspace', error);
              return false;
          } finally {
              isRestoringWorkspace = false;
          }
      }

      function sqlRunHistoryUrl() {
          return `/sql/run-history/${encodeURIComponent(connectionNickname)}?tab=${encodeURIComponent(activeSqlTabId)}`;
      }

      async function loadOutputHistory() {
          try {
              const resp = await fetch(sqlRunHistoryUrl());
              if (!resp.ok) throw new Error(await resp.text());
              const history = await resp.json();
              if (!Array.isArray(history)) return [];
              outputHistoryCache = history
                  .filter((entry) => entry.status === 'completed' && entry.html)
                  .map((entry, index) => ({
                      ...entry,
                      createdAt: entry.created_at || entry.createdAt,
                      queryName: entry.query_name || entry.queryName || '',
                      queryFolder: entry.query_folder || entry.queryFolder || '',
                      rowCountText: entry.row_count_text || entry.rowCountText || '',
                      id: entry.id || `${entry.created_at || 'history'}-${index}`
                  }));
              return outputHistoryCache;
          } catch (e) {
              console.error('Failed to load SQL output history', e);
              return outputHistoryCache;
          }
      }

      async function saveOutputHistoryEntry(entry) {
          try {
              const resp = await fetch('/sql/run-history', {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({
                      id: entry.id,
                      connection: connectionNickname,
                      tab_id: activeSqlTabId,
                      sql: entry.sql,
                      query_name: entry.queryName || '',
                      query_folder: entry.queryFolder || '',
                      status: entry.status || 'completed',
                      created_at: entry.createdAt,
                      completed_at: entry.completedAt || entry.createdAt,
                      row_count_text: entry.rowCountText || '',
                      html: entry.html || '',
                      error: entry.error || null,
                  }),
              });
              if (!resp.ok) throw new Error(await resp.text());
              saveSqlWorkspaceState();
          } catch (e) {
              console.error('Failed to save SQL output history', e);
          }
      }

      function getOutputRowCount() {
          const table = getActiveOutputTable();
          if (!table) return '';

          const rows = getTableBodyRows(table);
          return rows.length + " rows";
      }

      function applyOutputHtml(html, rowCountText = '') {
          output.innerHTML = html;
          if (outputFilterInput) {
              outputFilterInput.value = '';
          }

          const table = getActiveOutputTable();
          if (table) {
              makeTableInteractable(table);
          }

          const countSpan = document.getElementById('row-count');
          if (countSpan) {
              countSpan.innerText = rowCountText || getOutputRowCount();
          }
          updateSelectionCount();
          saveSqlWorkspaceState();
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
          outputHistorySelect.innerHTML = '<option value="">Output history</option>';
          outputHistoryCache.forEach((entry) => {
              const option = document.createElement('option');
              option.value = entry.id;
              option.textContent = outputHistoryLabel(entry);
              outputHistorySelect.appendChild(option);
          });
          if (nextSelectedId && outputHistoryCache.some((entry) => entry.id === nextSelectedId)) {
              outputHistorySelect.value = nextSelectedId;
          }
      }

      async function refreshOutputHistoryOptions(selectedId = '') {
          await loadOutputHistory();
          renderOutputHistoryOptions(selectedId);
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

      async function cacheOutputHistory(html, sql, existingId = '', createdAt = '', rowCountText = '') {
          if (!html || html.length > maxOutputHistoryEntryChars) return;

          const entry = {
              id: existingId || String(Date.now()) + "-" + Math.random().toString(16).slice(2),
              createdAt: createdAt || new Date().toISOString(),
              completedAt: new Date().toISOString(),
              sql: sql,
              queryName: queryNameInput.value.trim(),
              queryFolder: queryFolderInput ? queryFolderInput.value : '',
              rowCountText: rowCountText || currentRowCountText() || getOutputRowCount(),
              status: 'completed',
              html: html
          };

          const history = outputHistoryCache.filter((existing) => existing.id !== entry.id && (existing.html !== html || existing.sql !== sql));
          history.unshift(entry);
          outputHistoryCache = pruneOutputHistory(history);
          await saveOutputHistoryEntry(entry);
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
          if (!job || job.status !== 'completed') return;
          const html = job.html || '<pre>Query completed. 0 rows returned.</pre>';
          applyOutputHtml(html, job.row_count_text || '0 rows');
          editor.value = job.sql || editor.value;
          queryNameInput.value = job.query_name || '';
          if (queryFolderInput) queryFolderInput.value = job.query_folder || '';
          scanForVariables();
          handleInput();
          await cacheOutputHistory(html, job.sql || editor.value, job.id, job.created_at, job.row_count_text || '');
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

          return outputHistoryCache.find((entry) => entry.id === selectedId) || null;
      }

      refreshOutputHistoryOptions();

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
          deleteOutputHistoryBtn.addEventListener('click', async () => {
              if (!outputHistorySelect || outputHistorySelect.value === '') return;
              if (!window.confirm('Delete the selected SQL output history entry?')) return;

              const selectedId = outputHistorySelect.value;
              const resp = await fetch(`/sql/run-history/${encodeURIComponent(selectedId)}`, { method: 'DELETE' });
              if (!resp.ok) {
                  console.error('Failed to delete SQL history', await resp.text());
                  return;
              }
              outputHistoryCache = outputHistoryCache.filter((entry) => entry.id !== selectedId);
              renderOutputHistoryOptions();
          });
      }

      if (clearOutputHistoryBtn) {
          clearOutputHistoryBtn.addEventListener('click', async () => {
              if (!window.confirm('Clear all cached SQL output history for this connection?')) return;

              const resp = await fetch(`/sql/run-history/connection/${encodeURIComponent(connectionNickname)}?tab=${encodeURIComponent(activeSqlTabId)}`, { method: 'DELETE' });
              if (!resp.ok) {
                  console.error('Failed to clear SQL history', await resp.text());
                  return;
              }
              outputHistoryCache = [];
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

      function getVisibleColumnNames(table) {
          const headers = Array.from(table.querySelectorAll('thead th'));
          return getVisibleColumnIndexes(table)
              .map((index) => getCleanHeaderText(headers[index], `Column ${index + 1}`))
              .filter(Boolean);
      }

      function rewriteEditorSelectColumns(table) {
          const visibleColumns = getVisibleColumnNames(table);
          if (visibleColumns.length === 0) return;

          const nextSelectList = visibleColumns
              .map((column) => `    ${quoteSqlIdentifierForDisplay(column)}`)
              .join(',\n');
          const currentSql = editor.value || '';
          const selectMatch = currentSql.match(/^(\s*SELECT\s+(?:DISTINCT\s+)?)([\s\S]*?)(\s+FROM\s+[\s\S]*)$/i);

          if (!selectMatch) return;

          editor.value = `${selectMatch[1]}\n${nextSelectList}${selectMatch[3]}`;
          scanForVariables();
          handleInput();
          saveSqlWorkspaceState();
      }

      function getExportRows(table, rowsMode) {
          const visibleBodyRows = getTableBodyRows(table)
              .filter((row) => !row.hidden && row.style.display !== 'none');
          if (rowsMode === 'selected') {
              return visibleBodyRows.filter((row) => row.classList.contains('selected-row'));
          }
          return visibleBodyRows;
      }

      async function exportCurrentResults(mode) {
          const table = getActiveOutputTable();
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

          const filename = `${connectionNickname || 'sql'}-${rowsMode}-results.csv`;
          try {
              await saveGeneratedFile(filename, csvContent, 'text/csv');
          } catch (error) {
              if (error?.name === 'AbortError') return;
              alert(`Failed to export CSV: ${error.message || error}`);
          }
      }

      if (exportMenuPanel) {
          exportMenuPanel.addEventListener('click', async (event) => {
              const button = event.target.closest('[data-export-mode]');
              if (!button) return;
              setMenuOpen(exportMenuBtn, exportMenuPanel, false);
              await exportCurrentResults(button.dataset.exportMode || 'all-headers');
          });
      }
      
      // Filtering Logic
      outputFilterInput.addEventListener('input', (e) => {
          const term = e.target.value.toLowerCase();
          const table = getActiveOutputTable();
          if(!table) return;
          const rows = getTableBodyRows(table);
          
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

      if (clearOutputBtn) {
          clearOutputBtn.addEventListener('click', () => {
              resetSqlOutputPane();
              saveSqlWorkspaceState();
          });
      }

      function attachStickyResultHeader(table) {
          const scrollRoot = table.closest('.table-browser-results') || output;
          if (!scrollRoot) return;

          if (scrollRoot._sqlStickyHeaderCleanup) {
              scrollRoot._sqlStickyHeaderCleanup();
          }
          scrollRoot.querySelectorAll('.sql-sticky-header-clone').forEach((element) => element.remove());

          const sourceThead = table.querySelector('thead');
          if (!sourceThead) return;

          const sticky = document.createElement('div');
          sticky.className = 'sql-sticky-header-clone';
          sticky.hidden = true;

          const stickyTable = document.createElement('table');
          stickyTable.className = table.className;
          stickyTable.classList.add('sql-sticky-header-table');
          sticky.appendChild(stickyTable);
          scrollRoot.insertBefore(sticky, scrollRoot.firstChild);

          const syncStickyHeader = () => {
              const sourceColgroup = table.querySelector('colgroup');
              const clonedColgroup = sourceColgroup ? sourceColgroup.cloneNode(true) : null;
              const clonedThead = sourceThead.cloneNode(true);
              const sourceHeaders = Array.from(sourceThead.querySelectorAll('th'));

              stickyTable.innerHTML = '';
              if (clonedColgroup) stickyTable.appendChild(clonedColgroup);
              stickyTable.appendChild(clonedThead);
              stickyTable.style.width = table.offsetWidth + 'px';
              stickyTable.style.transform = scrollRoot.scrollLeft ? `translateX(${-scrollRoot.scrollLeft}px)` : '';

              clonedThead.querySelectorAll('th').forEach((stickyTh, index) => {
                  const sourceTh = sourceHeaders[index];
                  if (!sourceTh) return;
                  stickyTh.dataset.columnIndex = sourceTh.dataset.columnIndex || String(index);

                  const handle = stickyTh.querySelector('.column-resize-handle');
                  if (handle) {
                      handle.addEventListener('mousedown', (event) => {
                          if (typeof table._startColumnResize === 'function') {
                              table._startColumnResize(index, event, handle);
                          }
                      });
                  }

                  stickyTh.addEventListener('click', (event) => {
                      if (event.target.closest('.column-resize-handle')) return;
                      sourceTh.click();
                      syncStickyHeader();
                  });
              });

              const tableRect = table.getBoundingClientRect();
              const rootRect = scrollRoot.getBoundingClientRect();
              const isScrolledPastHeader = tableRect.top < rootRect.top;
              const isTableStillVisible = tableRect.bottom > rootRect.top;
              sticky.hidden = !(isScrolledPastHeader && isTableStillVisible);
          };

          const onScroll = () => syncStickyHeader();
          const onResize = () => syncStickyHeader();
          scrollRoot.addEventListener('scroll', onScroll, { passive: true });
          window.addEventListener('resize', onResize);
          scrollRoot._sqlStickyHeaderCleanup = () => {
              scrollRoot.removeEventListener('scroll', onScroll);
              window.removeEventListener('resize', onResize);
              sticky.remove();
              delete scrollRoot._sqlStickyHeaderCleanup;
          };
          table._syncStickyHeader = syncStickyHeader;
          syncStickyHeader();
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
        } else {
            while (colgroup.children.length < ths.length) {
                const col = document.createElement('col');
                col.style.width = minColumnWidth + 'px';
                colgroup.appendChild(col);
            }
        }
        
        rows.forEach((row, i) => {
            row.dataset.originalIndex = i;
        });

        tbody.addEventListener('click', (e) => {
            const tr = e.target.closest('tr');
            if (!tr) return;
            if (e.target.closest('button, input, textarea, select, a, [contenteditable="true"]')) return;
            if (window.getSelection && window.getSelection().toString().trim() !== '') return;

            tr.classList.toggle('selected-row');
            updateSelectionCount();
        });

        let currentSortCol = -1;
        let currentSortDir = 'none'; 

        const startColumnResize = (colIndex, event, activeHandle) => {
            event.preventDefault();
            event.stopPropagation();

            const col = colgroup.children[colIndex];
            const sourceTh = ths[colIndex];
            const startX = event.clientX;
            const startWidth = col?.getBoundingClientRect().width || sourceTh?.getBoundingClientRect().width || minColumnWidth;
            activeHandle.classList.add('resizing');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';

            const onMouseMove = (moveEvent) => {
                const nextWidth = Math.max(minColumnWidth, startWidth + moveEvent.clientX - startX);
                if (col) {
                    col.style.width = nextWidth + 'px';
                }
                if (table._syncStickyHeader) table._syncStickyHeader();
            };

            const onMouseUp = () => {
                activeHandle.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                document.removeEventListener('mousemove', onMouseMove);
                document.removeEventListener('mouseup', onMouseUp);
                if (table._syncStickyHeader) table._syncStickyHeader();
            };

            document.addEventListener('mousemove', onMouseMove);
            document.addEventListener('mouseup', onMouseUp);
        };

        table._startColumnResize = startColumnResize;

        ths.forEach((th, colIndex) => {
            th.dataset.columnIndex = String(colIndex);
            const sortIndicator = document.createElement('span');
            sortIndicator.className = 'column-sort-indicator';
            th.appendChild(sortIndicator);

            const handle = document.createElement('span');
            handle.className = 'column-resize-handle';
            handle.title = 'Drag to resize column';
            th.appendChild(handle);

            handle.addEventListener('mousedown', (event) => startColumnResize(colIndex, event, handle));

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
        attachStickyResultHeader(table);
      }

      function setColumnVisibility(table, colIndex, isVisible) {
          const cells = table.querySelectorAll(`thead th:nth-child(${colIndex + 1}), tbody td:nth-child(${colIndex + 1})`);
          cells.forEach((cell) => {
              cell.classList.toggle('sql-column-hidden', !isVisible);
          });
          const col = table.querySelector('colgroup')?.children[colIndex];
          if (col) {
              col.classList.toggle('sql-column-hidden', !isVisible);
              col.style.visibility = isVisible ? '' : 'collapse';
              col.style.width = isVisible && !col.style.width ? 'auto' : col.style.width;
          }
          if (table._syncStickyHeader) table._syncStickyHeader();
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
                  if (!checkbox.checked && getVisibleColumnIndexes(table).length <= 1) {
                      checkbox.checked = true;
                      return;
                  }
                  setColumnVisibility(table, colIndex, checkbox.checked);
                  rewriteEditorSelectColumns(table);
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
          
          const btn = variablesLeft.querySelector('.add-var-btn');
          variablesLeft.insertBefore(div, btn);
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
          
          const currentInputs = Array.from(variablesLeft.querySelectorAll('input'));
          const currentValues = {};
          currentInputs.forEach(i => { if(i.name) currentValues[i.name] = i.value; });
          
          const existingGroups = variablesLeft.querySelectorAll('.var-input-group');
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
          saveSqlWorkspaceState();
      };

      function toggleSqlLineComment() {
          const value = editor.value;
          const selectionStart = editor.selectionStart;
          const selectionEnd = editor.selectionEnd;
          const hasSelection = selectionEnd > selectionStart;
          const blockStart = value.lastIndexOf('\n', selectionStart - 1) + 1;
          const adjustedEnd = hasSelection && value[selectionEnd - 1] === '\n'
              ? selectionEnd - 1
              : selectionEnd;
          const nextLineBreak = value.indexOf('\n', adjustedEnd);
          const blockEnd = nextLineBreak === -1 ? value.length : nextLineBreak;
          const block = value.slice(blockStart, blockEnd);
          const lines = block.split('\n');
          const meaningfulLines = lines.filter((line) => line.trim().length > 0);
          const shouldUncomment = meaningfulLines.length > 0
              ? meaningfulLines.every((line) => /^\s*-- ?/.test(line))
              : /^\s*-- ?/.test(lines[0] || '');
          const nextLines = lines.map((line) => {
              if (shouldUncomment) return line.replace(/^(\s*)-- ?/, '$1');
              return line.replace(/^(\s*)/, '$1-- ');
          });
          const nextBlock = nextLines.join('\n');
          const nextValue = value.slice(0, blockStart) + nextBlock + value.slice(blockEnd);
          const delta = nextBlock.length - block.length;

          editor.value = nextValue;
          if (hasSelection) {
              editor.setSelectionRange(blockStart, blockEnd + delta);
          } else {
              const nextCaret = Math.max(blockStart, selectionStart + delta);
              editor.setSelectionRange(nextCaret, nextCaret);
          }
          handleInput();
          syncScroll();
      }

      const syncScroll = () => {
          backdrop.scrollTop = editor.scrollTop;
          backdrop.scrollLeft = editor.scrollLeft;
      };

      editor.addEventListener('input', handleInput);
      editor.addEventListener('scroll', syncScroll);
      editor.addEventListener('keydown', (event) => {
          const isCommentShortcut = (event.ctrlKey || event.metaKey) && !event.altKey && event.code === 'Slash';
          if (!isCommentShortcut) return;
          event.preventDefault();
          toggleSqlLineComment();
      });
      queryNameInput.addEventListener('input', saveSqlWorkspaceState);
      if (queryFolderInput) {
          queryFolderInput.addEventListener('change', saveSqlWorkspaceState);
      }

      const restoredWorkspace = restoreSqlWorkspaceState();
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

      if (!restoredWorkspace && editor.value === "") { editor.value = "SELECT 1;"; handleInput(); }
