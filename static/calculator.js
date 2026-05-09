        const display = document.getElementById('current-input');
        const historyList = document.getElementById('history-list');
        const modeToggleBtn = document.getElementById('mode-toggle');
        const scientificBtns = document.getElementById('scientific-buttons');
        const buttonsWrapper = document.querySelector('.buttons-wrapper'); 
        
        let currentExpression = '0';
        let history = [];
        let isScientificMode = false;

        const keyMap = {
            'Enter': 'Enter', 'Escape': 'c', 'Delete': 'c', '/': '/', '*': '*', '-': '-', '+': '+', '.': '.', '^': '^', 
            'p': 'π', 'e': 'e', '!': '!', '(': '(', ')': ')', 'Backspace': 'backspace',
        };
        for (let i = 0; i <= 9; i++) { keyMap[i.toString()] = i.toString(); }

        function updateDisplay() {
            if (display) display.textContent = currentExpression === '' ? '0' : currentExpression;
        }
        
        function factorial(n) {
            if (n < 0 || n !== Math.floor(n)) return NaN;
            if (n === 0) return 1;
            let result = 1;
            for (let i = 2; i <= n; i++) { result *= i; }
            return result;
        }

        function prepareExpression(expression) {
            let prepared = expression
                .replace(/×/g, '*').replace(/÷/g, '/').replace(/%/g, '/100*').replace(/\^/g, '**')
                .replace(/sin\(([^)]+)\)/g, (match, p1) => `Math.sin((${p1}) * (Math.PI / 180))`) 
                .replace(/cos\(([^)]+)\)/g, (match, p1) => `Math.cos((${p1}) * (Math.PI / 180))`) 
                .replace(/tan\(([^)]+)\)/g, (match, p1) => `Math.tan((${p1}) * (Math.PI / 180))`) 
                .replace(/log\(([^)]+)\)/g, 'Math.log10($1)').replace(/ln\(([^)]+)\)/g, 'Math.log($1)')
                .replace(/sqrt\(([^)]+)\)/g, 'Math.sqrt($1)').replace(/π/g, 'Math.PI').replace(/e/g, 'Math.E');
            prepared = prepared.replace(/(\d+)!/g, (match, p1) => `factorial(${p1})`);
            prepared = prepared.replace(/[\+\-\*\/%]+$/, '');
            return prepared;
        }

        function calculate() {
            let openCount = (currentExpression.match(/\(/g) || []).length;
            let closeCount = (currentExpression.match(/\)/g) || []).length;
            let expressionToEvaluate = currentExpression;
            if (openCount > closeCount) {
                const missingClosers = openCount - closeCount;
                for (let i = 0; i < missingClosers; i++) { expressionToEvaluate += ')'; }
            }
            try {
                const expressionToEval = prepareExpression(expressionToEvaluate);
                const result = Function(`'use strict'; const factorial = ${factorial.toString()}; return (${expressionToEval});`)();
                if (result === undefined || isNaN(result) || !isFinite(result)) { throw new Error("Invalid calculation"); }
                const formattedResult = parseFloat(result.toFixed(10)).toString();
                addToHistory(currentExpression, formattedResult);
                currentExpression = formattedResult;
            } catch (e) {
                currentExpression = 'Error';
                setTimeout(() => { currentExpression = '0'; updateDisplay(); }, 1500);
            }
            updateDisplay();
        }

        function addToHistory(expression, result) {
            if (history.length > 0 && history[0].expression === expression && history[0].result === result) return; 
            history.unshift({ expression, result });
            if (history.length > 10) history.pop();
            renderHistory();
        }

        function renderHistory() {
            if (!historyList) return;
            historyList.innerHTML = '';
            if (history.length === 0) {
                historyList.innerHTML = '<li id="history-empty">No history yet.</li>';
                return;
            }
            history.forEach((item, index) => {
                const li = document.createElement('li');
                li.innerHTML = `<div class="history-expression">${item.expression} =</div><div class="history-result">${item.result}</div>`;
                li.addEventListener('click', () => { currentExpression = item.result; updateDisplay(); });
                historyList.appendChild(li);
            });
        }

        function handleInput(key) {
            const operators = ['+', '-', '×', '÷', '%', '^'];
            const lastChar = currentExpression.slice(-1);
            const isFunction = ['sin', 'cos', 'tan', 'log', 'ln', 'sqrt'].includes(key);
            if (currentExpression === 'Error' && key !== 'c') return;

            switch (key) {
                case 'c': currentExpression = '0'; break;
                case 'backspace': currentExpression = currentExpression.length <= 1 || currentExpression.endsWith('Error') ? '0' : currentExpression.slice(0, -1); break;
                case 'Enter': 
                case '=': if (currentExpression !== 'Error' && !operators.includes(lastChar) && !['(', '.'].includes(lastChar)) { calculate(); } break;
                case 'π':
                case 'e':
                    if (currentExpression === '0') currentExpression = key;
                    else if (/\d$|\)$/.test(lastChar)) currentExpression += '*' + key;
                    else if (operators.includes(lastChar) || lastChar === '(') currentExpression += key;
                    else currentExpression = key; 
                    break;
                case '(':
                    if (currentExpression === '0' || operators.includes(lastChar)) currentExpression += key;
                    else if (/\d$/.test(lastChar) || lastChar === ')') currentExpression += '*' + key;
                    else currentExpression += key;
                    break;
                case ')': if (currentExpression.includes('(') && !operators.includes(lastChar) && lastChar !== '(') currentExpression += key; break;
                case '!': if (/\d$|\)$/.test(currentExpression)) currentExpression += key; break;
                case '+':
                case '-':
                case '*':
                case '/':
                case '%':
                case '^':
                    const op = key === '*' ? '×' : (key === '/' ? '÷' : key);
                    if (operators.includes(lastChar)) currentExpression = currentExpression.slice(0, -1) + op;
                    else if (currentExpression === '0') { if (key === '-') currentExpression = '-'; }
                    else currentExpression += op;
                    break;
                case '.':
                    const parts = currentExpression.split(/[\+\-×÷%\^]/);
                    const lastNum = parts[parts.length - 1];
                    if (!lastNum.includes('.') && !lastNum.endsWith('π') && !lastNum.endsWith('e')) currentExpression += key;
                    break;
                default:
                    if (isFunction) {
                         if (currentExpression === '0') currentExpression = `${key}(`;
                         else if (operators.includes(lastChar) || lastChar === '(') currentExpression += `${key}(`;
                         else if (/\d$|\)$/.test(lastChar)) currentExpression += `*${key}(`;
                         else currentExpression = `${key}(`;
                    } else {
                        if (/\) $|[πe]$/.test(lastChar)) currentExpression += '*' + key;
                        else if (currentExpression === '0') currentExpression = key;
                        else currentExpression += key;
                    }
                    break;
            }
            updateDisplay();
        }
        
        function toggleMode() {
            isScientificMode = !isScientificMode;
            scientificBtns.style.display = isScientificMode ? 'grid' : 'none';
            modeToggleBtn.textContent = isScientificMode ? 'Switch to Standard' : 'Switch to Scientific';
        }

        if (buttonsWrapper) {
            buttonsWrapper.addEventListener('click', (event) => {
                const button = event.target.closest('.calc-button');
                if (button && button.id !== 'mode-toggle') handleInput(button.getAttribute('data-key'));
            });
        }

        if (modeToggleBtn) modeToggleBtn.addEventListener('click', toggleMode);

        document.addEventListener('keydown', (event) => {
            if (document.activeElement.tagName === 'INPUT' || document.activeElement.tagName === 'TEXTAREA') return;
            const key = event.key;
            if (key in keyMap) { event.preventDefault(); handleInput(keyMap[key]); }
            else if (key === 'Delete') { event.preventDefault(); handleInput('c'); }
        });
        
        updateDisplay();
        renderHistory();

        // Drag logic for floating calculator
        const calcWindow = document.getElementById('floating-calculator');
        const calcHandle = document.getElementById('calc-drag-handle');
        let isDragging = false;
        let currentX = 0;
        let currentY = 0;
        let initialX = 0;
        let initialY = 0;
        let xOffset = 0;
        let yOffset = 0;

        function setTranslate(xPos, yPos, el) {
            if (el) el.style.transform = `translate3d(${xPos}px, ${yPos}px, 0)`;
        }

        function resetCalculatorPosition() {
            xOffset = 0;
            yOffset = 0;
            currentX = 0;
            currentY = 0;
            setTranslate(0, 0, calcWindow);
            localStorage.setItem('calc-pos', JSON.stringify({x: 0, y: 0}));
        }

        function ensureCalculatorInViewport() {
            if (!calcWindow) return;
            requestAnimationFrame(() => {
                const rect = calcWindow.getBoundingClientRect();
                const edge = 24;
                const isUsable = rect.right > edge
                    && rect.bottom > edge
                    && rect.left < window.innerWidth - edge
                    && rect.top < window.innerHeight - edge;

                if (!isUsable) {
                    resetCalculatorPosition();
                }
            });
        }

        if (calcWindow && calcHandle) {
            // Restore position from localStorage
            const savedPos = localStorage.getItem('calc-pos');
            if (savedPos) {
                try {
                    const {x, y} = JSON.parse(savedPos);
                    xOffset = Number(x) || 0;
                    yOffset = Number(y) || 0;
                    setTranslate(xOffset, yOffset, calcWindow);
                } catch (_) {
                    localStorage.removeItem('calc-pos');
                }
            }

            calcHandle.addEventListener('mousedown', dragStart);
            calcWindow.addEventListener('dblclick', centerOnEdgeDoubleClick);
            document.addEventListener('mousemove', drag);
            document.addEventListener('mouseup', dragEnd);

            function isOuterEdgeClick(e) {
                if (e.target.closest('button, input, textarea, select, a')) return false;
                const rect = calcWindow.getBoundingClientRect();
                const edge = 18;
                const onEdge = e.clientX - rect.left <= edge ||
                    rect.right - e.clientX <= edge ||
                    e.clientY - rect.top <= edge ||
                    rect.bottom - e.clientY <= edge;
                return onEdge || calcHandle.contains(e.target);
            }

            function centerOnEdgeDoubleClick(e) {
                if (!isOuterEdgeClick(e)) return;

                const rect = calcWindow.getBoundingClientRect();
                const targetLeft = Math.max(0, (window.innerWidth - rect.width) / 2);
                const targetTop = Math.max(0, (window.innerHeight - rect.height) / 2);
                xOffset += targetLeft - rect.left;
                yOffset += targetTop - rect.top;
                currentX = xOffset;
                currentY = yOffset;
                setTranslate(xOffset, yOffset, calcWindow);
                localStorage.setItem('calc-pos', JSON.stringify({x: xOffset, y: yOffset}));
            }

            function dragStart(e) {
                initialX = e.clientX - xOffset;
                initialY = e.clientY - yOffset;
                if (e.target === calcHandle || calcHandle.contains(e.target)) isDragging = true;
            }
            function drag(e) {
                if (isDragging) {
                    e.preventDefault();
                    currentX = e.clientX - initialX;
                    currentY = e.clientY - initialY;
                    xOffset = currentX; yOffset = currentY;
                    setTranslate(currentX, currentY, calcWindow);
                }
            }
            function dragEnd() {
                initialX = currentX; initialY = currentY;
                isDragging = false;
                localStorage.setItem('calc-pos', JSON.stringify({x: xOffset, y: yOffset}));
            }
        }

        function toggleCalculatorInPage() {
            const calc = document.getElementById('floating-calculator');
            if (!calc) return;
            const isVisible = calc.style.display === 'flex';
            calc.style.display = isVisible ? 'none' : 'flex';
            if (!isVisible && window.bringFloatingWindowToFront) {
                window.bringFloatingWindowToFront(calc);
            }
            if (!isVisible) {
                ensureCalculatorInViewport();
            }
            localStorage.setItem('calc-visible', !isVisible);
        }

        window.toggleCalculator = function() {
            if (window.openDesktopToolWindow?.('calculator', toggleCalculatorInPage)) return;
            toggleCalculatorInPage();
        };

        // Restore visibility
        if (localStorage.getItem('calc-visible') === 'true') {
            const calc = document.getElementById('floating-calculator');
            if (calc) {
                calc.style.display = 'flex';
                ensureCalculatorInViewport();
            }
        }
