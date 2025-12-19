use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;

use crate::app_state::AppState;
use crate::base_page::render_base_page;

// Handler for GET /calculator
#[get("/calculator")]
pub async fn calculator_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();
    
    let content = r#"
        <div style="padding: 50px; text-align: center; max-width: 600px; margin: 0 auto;">
            <h1>Calculator</h1>
            <p style="font-size: 1.1em; opacity: 0.8; margin-bottom: 30px;">
                The calculator is now a floating app that stays with you! 
                You can toggle it from anywhere using the <b>Calculator</b> button in the navigation bar.
            </p>
            <button class="form-submit-btn" onclick="toggleCalculator()" style="width: auto; padding: 12px 30px; font-size: 1.1em; cursor: pointer;">
                Open Calculator
            </button>
            <p style="margin-top: 40px; font-size: 0.9em; opacity: 0.6;">
                It remembers its position and visibility state even when you switch pages or refresh.
            </p>
        </div>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_base_page("Calculator", content, &current_theme, &saved_themes))
}

pub fn get_css() -> String {
    r#"
<style>
    /* Floating Calculator Container */
    #floating-calculator {
        position: fixed;
        top: 100px;
        right: 20px;
        width: 350px;
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
        z-index: 1000;
        display: none; /* Hidden by default */
        flex-direction: column;
        user-select: none;
        overflow: hidden;
    }

    .calc-header {
        background-color: var(--tertiary-bg);
        padding: 8px 12px;
        cursor: move;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px solid var(--border-color);
        font-weight: bold;
        font-size: 0.9em;
    }

    .calc-close-btn {
        background: none;
        border: none;
        color: var(--text-color);
        font-size: 1.2em;
        cursor: pointer;
        opacity: 0.6;
        padding: 0 5px;
    }
    .calc-close-btn:hover { opacity: 1; color: #f44336; }

    .calculator-app {
        display: flex;
        padding: 10px;
        flex-direction: column;
        gap: 10px;
    }
    
    .calculator-container {
        display: flex;
        flex-direction: column;
    }
    .mode-toggle {
        margin-bottom: 8px;
        padding: 5px 10px !important;
        font-size: 0.8em !important;
    }
    .display {
        background-color: var(--primary-bg);
        color: var(--text-color);
        padding: 10px;
        margin-bottom: 10px;
        font-size: 1.8em;
        text-align: right;
        border-radius: 4px;
        min-height: 40px;
        overflow-x: auto;
        white-space: nowrap;
        line-height: 1.2;
    }
    .current-input {
        font-size: 1.2em;
        color: var(--text-color);
        min-height: 25px;
    }
    .buttons-wrapper {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    
    .scientific-buttons {
        display: none; 
        grid-template-columns: repeat(4, 1fr); 
        gap: 5px;
    }
    
    .standard-buttons {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 5px;
    }
    
    .calc-button {
        background-color: var(--tertiary-bg);
        color: var(--text-color);
        border: none;
        padding: 12px 5px;
        font-size: 1em;
        border-radius: 4px;
        cursor: pointer;
        transition: background-color 0.1s;
        box-shadow: 0 1px var(--border-color);
    }
    .calc-button:active {
        box-shadow: none;
        transform: translateY(1px);
    }
    .calc-button.operator {
        background-color: var(--link-color);
        color: var(--primary-bg);
    }
    .calc-button.scientific {
        background-color: var(--tertiary-bg);
        opacity: 0.8;
    }
    .calc-button.clear {
        background-color: #d33;
        color: white;
    }
    .calc-button.equals {
        background-color: #4CAF50;
        color: white;
        grid-column: span 2;
    }
    
    .history-container {
        border-top: 1px solid var(--border-color);
        padding-top: 10px;
        margin-top: 5px;
        max-height: 150px;
        overflow-y: auto;
    }
    .history-container h2 {
        margin: 0 0 5px 0;
        font-size: 0.9em;
        opacity: 0.7;
    }
    #history-list {
        list-style: none;
        padding: 0;
        margin: 0;
    }
    #history-list li {
        border-bottom: 1px solid var(--border-color);
        padding: 4px 0;
        font-size: 0.8em;
        cursor: pointer;
    }
    #history-list li:hover { background-color: var(--tertiary-bg); }
    .history-expression { color: #888; }
    .history-result { font-weight: bold; }

    /* For standalone page compatibility */
    body:not(:has(.modern-nav)) #floating-calculator {
        position: static;
        display: flex;
        margin: 20px auto;
        width: 400px;
    }
</style>
    "#.to_string()
}

pub fn get_html() -> String {
    r#"
    <div id="floating-calculator">
        <div class="calc-header" id="calc-drag-handle">
            <span>Calculator</span>
            <button class="calc-close-btn" id="calc-close-btn" onclick="toggleCalculator()">&times;</button>
        </div>
        <div class="calculator-app">
            <div class="calculator-container">
                <button id="mode-toggle" class="calc-button mode-toggle">Switch to Scientific</button>
                <div class="display">
                    <div id="current-input" class="current-input">0</div>
                </div>
                
                <div class="buttons-wrapper">
                    <div id="scientific-buttons" class="scientific-buttons">
                        <button class="calc-button scientific" data-key="sin">sin</button>
                        <button class="calc-button scientific" data-key="cos">cos</button>
                        <button class="calc-button scientific" data-key="tan">tan</button>
                        <button class="calc-button scientific" data-key="log">log</button>
                        <button class="calc-button scientific" data-key="ln">ln</button>
                        <button class="calc-button scientific" data-key="^">xʸ</button>
                        <button class="calc-button scientific" data-key="sqrt">√</button>
                        <button class="calc-button scientific" data-key="pi">π</button>
                        <button class="calc-button scientific" data-key="e">e</button>
                        <button class="calc-button scientific" data-key="!">x!</button>
                        <button class="calc-button scientific" data-key="(">(</button>
                        <button class="calc-button scientific" data-key=")">)</button>
                    </div>

                    <div id="standard-buttons" class="standard-buttons">
                        <button class="calc-button clear" data-key="c">C</button>
                        <button class="calc-button operator" data-key="backspace">⌫</button>
                        <button class="calc-button operator" data-key="%">%</button>
                        <button class="calc-button operator" data-key="/">÷</button>
                        <button class="calc-button" data-key="7">7</button>
                        <button class="calc-button" data-key="8">8</button>
                        <button class="calc-button" data-key="9">9</button>
                        <button class="calc-button operator" data-key="*">×</button>
                        <button class="calc-button" data-key="4">4</button>
                        <button class="calc-button" data-key="5">5</button>
                        <button class="calc-button" data-key="6">6</button>
                        <button class="calc-button operator" data-key="-">-</button>
                        <button class="calc-button" data-key="1">1</button>
                        <button class="calc-button" data-key="2">2</button>
                        <button class="calc-button" data-key="3">3</button>
                        <button class="calc-button operator" data-key="+">+</button>
                        <button class="calc-button" data-key="0">0</button>
                        <button class="calc-button" data-key=".">.</button>
                        <button class="calc-button equals" data-key="Enter">=</button>
                    </div>
                </div>
            </div>

            <div class="history-container">
                <h2>History</h2>
                <ul id="history-list">
                    <li id="history-empty">No history yet.</li>
                </ul>
            </div>
        </div>
    </div>
    "#.to_string()
}

pub fn get_js() -> String {
    r#"
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
        if (calcWindow && calcHandle) {
            let isDragging = false, currentX, currentY, initialX, initialY, xOffset = 0, yOffset = 0;
            
            // Restore position from localStorage
            const savedPos = localStorage.getItem('calc-pos');
            if (savedPos) {
                const {x, y} = JSON.parse(savedPos);
                xOffset = x; yOffset = y;
                setTranslate(x, y, calcWindow);
            }

            calcHandle.addEventListener('mousedown', dragStart);
            document.addEventListener('mousemove', drag);
            document.addEventListener('mouseup', dragEnd);

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
            function setTranslate(xPos, yPos, el) { el.style.transform = `translate3d(${xPos}px, ${yPos}px, 0)`; }
        }

        window.toggleCalculator = function() {
            const calc = document.getElementById('floating-calculator');
            if (!calc) return;
            const isVisible = calc.style.display === 'flex';
            calc.style.display = isVisible ? 'none' : 'flex';
            localStorage.setItem('calc-visible', !isVisible);
        };

        // Restore visibility
        if (localStorage.getItem('calc-visible') === 'true') {
            const calc = document.getElementById('floating-calculator');
            if (calc) calc.style.display = 'flex';
        }
    "#.to_string()
}