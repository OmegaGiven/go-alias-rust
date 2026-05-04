const schemaTemplate = document.getElementById('sql-schema-data');
let dbSchema = schemaTemplate ? JSON.parse(schemaTemplate.textContent || '{}') : {};

const mainContent = document.getElementById('main');
      const editor = document.getElementById('sql-editor');
      const sidebarSearchInput = document.getElementById('sidebar-search-input');
      const sidebarTableList = document.getElementById('table-list');
      const querySearchInput = document.getElementById('query-search-input');
      const savedQueriesList = document.getElementById('saved-queries-list');
      const saveQueryForm = document.getElementById('save-query-form');
      const queryNameInput = document.getElementById('query-name');
      const querySqlInput = document.getElementById('query-sql');
      const variablesSection = document.getElementById('variables-section');
      const autocompleteList = document.getElementById('autocomplete-list');
      const saveSqlFileBtn = document.getElementById('save-sql-file-btn');
      let currentFocus = -1;
      
      const backdrop = document.getElementById('sql-backdrop');
      const highlights = backdrop.querySelector('.highlights');
      const connectionNickname = document.querySelector("input[name=connection]")?.value || "";
      const varsStorageKey = "sql_vars_" + connectionNickname;

      // --- Clear Button Logic ---
      const clearBtn = document.getElementById('clear-editor-btn');
      if (clearBtn) {
          clearBtn.addEventListener('click', () => {
              if(editor.value.trim() === '') return;
              editor.value = '';
              handleInput();
              editor.focus();
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

      function filterSavedQueries() {
          const filter = querySearchInput.value.toUpperCase();
          const listItems = savedQueriesList.getElementsByTagName('li');
          for (let i = 0; i < listItems.length; i++) {
              const itemText = listItems[i].querySelector('.query-link').textContent || listItems[i].querySelector('.query-link').innerText;
              if (itemText.toUpperCase().indexOf(filter) > -1) { listItems[i].style.display = 'flex'; } else { listItems[i].style.display = 'none'; }
          }
      }
      querySearchInput.addEventListener('keyup', filterSavedQueries);

      const form = document.getElementById('sql-form');
      const output = document.getElementById('output');
      
      form.addEventListener('submit', async (e) => {
        e.preventDefault();
        output.innerHTML = '<pre style="padding:10px;">Loading...</pre>';
        
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
            const resp = await fetch('/sql/run', { 
                method: 'POST', 
                headers: { 'Content-Type': 'application/json' }, 
                body: JSON.stringify(payload) 
            });
            const html = await resp.text();
            output.innerHTML = html;
            
            const table = output.querySelector('table');
            if(table) {
                makeTableInteractable(table);
                // UPDATE ROW COUNT
                const rows = table.querySelectorAll('tbody tr');
                const countSpan = document.getElementById('row-count');
                if(countSpan) countSpan.innerText = rows.length + " rows";
            }

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
      
      // Client-Side Export Logic
      document.getElementById('export-client-btn').addEventListener('click', () => {
          const table = output.querySelector('table');
          if(!table) return alert('No results to export');
          
          const includeHeaders = document.getElementById('export-headers').checked;
          const rows = Array.from(table.querySelectorAll('tr'));
          const selectedRows = Array.from(table.querySelectorAll('tr.selected-row'));
          
          // Use selected rows if any, otherwise all visible rows (respecting filter)
          let targetRows = selectedRows.length > 0 ? selectedRows : rows.filter(r => r.style.display !== 'none');
          
          // Ensure we don't duplicate headers if they happen to be selected or in the list
          // Actually, 'rows' includes the header row usually in thead. 
          // Let's grab headers separately.
          const theadRow = table.querySelector('thead tr');
          const tbodyRows = Array.from(table.querySelectorAll('tbody tr'));
          
          let csvContent = "";
          
          if(includeHeaders && theadRow) {
              const headers = Array.from(theadRow.children).map(th => `"${th.innerText.replace(/"/g, '""')}"`);
              csvContent += headers.join(",") + "\n";
          }
          
          // Filter body rows based on selection or visibility
          let rowsToExport = [];
          if(selectedRows.length > 0) {
               rowsToExport = selectedRows;
          } else {
               rowsToExport = tbodyRows.filter(r => r.style.display !== 'none');
          }
          
          rowsToExport.forEach(row => {
              const cols = Array.from(row.children).map(td => `"${td.innerText.replace(/"/g, '""')}"`);
              csvContent += cols.join(",") + "\n";
          });
          
          const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
          const link = document.createElement("a");
          const url = URL.createObjectURL(blob);
          link.setAttribute("href", url);
          link.setAttribute("download", "export.csv");
          link.style.visibility = 'hidden';
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
      });
      
      // Filtering Logic
      document.getElementById('output-filter').addEventListener('input', (e) => {
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

      let isSelecting = false;
      let selectionMode = true;

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
        const rows = Array.from(tbody.querySelectorAll('tr'));
        
        rows.forEach((row, i) => {
            row.dataset.originalIndex = i;
        });

        tbody.addEventListener('mousedown', (e) => {
            const tr = e.target.closest('tr');
            if (tr) {
                isSelecting = true;
                selectionMode = !tr.classList.contains('selected-row');
                tr.classList.toggle('selected-row', selectionMode);
                updateSelectionCount();
                e.preventDefault(); // Prevent text selection while dragging
            }
        });

        tbody.addEventListener('mouseover', (e) => {
            if (isSelecting) {
                const tr = e.target.closest('tr');
                if (tr) {
                    tr.classList.toggle('selected-row', selectionMode);
                    updateSelectionCount();
                }
            }
        });

        // Global mouseup to stop selection even if released outside table
        if (!window._selectionHandlerBound) {
            window.addEventListener('mouseup', () => {
                isSelecting = false;
            });
            window._selectionHandlerBound = true;
        }

        let currentSortCol = -1;
        let currentSortDir = 'none'; 

        ths.forEach((th, colIndex) => {
            th.addEventListener('click', () => {
                if (currentSortCol === colIndex) {
                    if (currentSortDir === 'none') currentSortDir = 'asc';
                    else if (currentSortDir === 'asc') currentSortDir = 'desc';
                    else currentSortDir = 'none';
                } else {
                    currentSortCol = colIndex;
                    currentSortDir = 'asc';
                }

                ths.forEach(h => h.innerHTML = h.innerHTML.replace(/ [▲▼]$/, '')); 
                if (currentSortDir === 'asc') th.innerHTML += ' ▲';
                if (currentSortDir === 'desc') th.innerHTML += ' ▼';

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
      }

      savedQueriesList.addEventListener('click', (e) => {
          const target = e.target.closest('a');
          if (target) { 
              e.preventDefault(); 
              const sql = target.getAttribute('data-sql'); 
              const name = target.getAttribute('data-name'); 
              editor.value = sql; 
              queryNameInput.value = name; 
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