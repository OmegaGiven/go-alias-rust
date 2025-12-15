use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;

use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

// Handler for GET /calculator
#[get("/calculator")]
pub async fn calculator_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_calculator_page(&current_theme))
}

fn render_calculator_page(current_theme: &Theme) -> String {
    let style = format!(
        r#"
<style>
    /* Calculator Custom Styles */
    .calculator-app {{
        display: flex;
        gap: 20px;
        max-width: 1000px; /* Increased max width for scientific mode */
        margin: 0 auto;
        padding: 20px;
        flex-direction: column;
    }}
    @media (min-width: 900px) {{ /* Increased breakpoint for scientific mode */
        .calculator-app {{
            flex-direction: row;
        }}
    }}
    .calculator-container {{
        flex: 3;
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
        padding: 15px;
        display: flex;
        flex-direction: column;
    }}
    .mode-toggle {{
        margin-bottom: 10px;
        align-self: flex-start;
    }}
    .display {{
        background-color: var(--primary-bg);
        color: var(--text-color);
        padding: 15px;
        margin-bottom: 10px;
        font-size: 2.5em;
        text-align: right;
        border-radius: 4px;
        min-height: 50px;
        overflow-x: auto;
        white-space: nowrap;
        line-height: 1.2;
    }}
    .current-input {{
        font-size: 1.5em;
        color: var(--text-color);
        min-height: 30px;
    }}
    .buttons-wrapper {{
        display: flex;
        gap: 10px;
        flex-grow: 1; /* Allow wrapper to grow */
    }}
    
    /* Scientific Buttons */
    .scientific-buttons {{
        display: none; /* Hidden by default */
        grid-template-columns: repeat(3, 1fr); 
        gap: 10px;
        flex: 0 0 30%; /* Fixed width for scientific functions */
    }}
    
    /* Standard Buttons */
    .standard-buttons {{
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 10px;
        flex: 1 1 65%; /* Take up the remaining space */
    }}
    
    .calc-button {{
        background-color: var(--tertiary-bg);
        color: var(--text-color);
        border: none;
        padding: 20px;
        font-size: 1.2em;
        border-radius: 6px;
        cursor: pointer;
        transition: background-color 0.1s, box-shadow 0.1s;
        box-shadow: 0 2px var(--border-color);
    }}
    .calc-button:active {{
        box-shadow: none;
        transform: translateY(2px);
    }}
    .calc-button.operator {{
        background-color: var(--link-color);
        color: var(--primary-bg);
    }}
    .calc-button.scientific {{
        background-color: #555; /* Darker background for scientific functions */
        color: var(--text-color);
    }}
    .calc-button.clear {{
        background-color: #d33;
        color: white;
    }}
    .calc-button.equals {{
        background-color: #4CAF50;
        color: white;
        grid-column: span 2;
    }}
    
    /* History Styles */
    .history-container {{
        flex: 1;
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 15px;
        max-height: 500px; /* Limit height for history */
        overflow-y: auto;
    }}
    @media (max-width: 899px) {{
        .history-container {{
            max-height: 250px; 
        }}
    }}
    .history-container h2 {{
        margin-top: 0;
        border-bottom: 1px solid var(--border-color);
        padding-bottom: 5px;
    }}
    #history-list {{
        list-style: none;
        padding: 0;
    }}
    #history-list li {{
        border-bottom: 1px dashed var(--border-color);
        padding: 8px 0;
        font-size: 0.9em;
        cursor: pointer;
    }}
    #history-list li:hover {{
        background-color: var(--tertiary-bg);
        border-radius: 4px;
        padding-left: 5px;
    }}
    .history-expression {{
        color: #aaa;
    }}
    .history-result {{
        font-weight: bold;
    }}
</style>
        "#,
    );

    let html_content = r#"
    <div class="calculator-app">
        <div class="calculator-container">
            <h1>Calculator</h1>
            <button id="mode-toggle" class="calc-button mode-toggle">Switch to Scientific</button>
            <div class="display">
                <div id="current-input" class="current-input">0</div>
            </div>
            
            <div class="buttons-wrapper">
                
                <!-- Scientific Buttons (Initially Hidden) -->
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

                <!-- Standard/Core Buttons -->
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
                <!-- History items will be inserted here by JavaScript -->
                <li id="history-empty">No history yet.</li>
            </ul>
        </div>
    </div>

    <script>
        const display = document.getElementById('current-input');
        const historyList = document.getElementById('history-list');
        const modeToggleBtn = document.getElementById('mode-toggle');
        const scientificBtns = document.getElementById('scientific-buttons');
        const buttonsWrapper = document.querySelector('.buttons-wrapper'); // NEW: Parent for delegation
        
        let currentExpression = '0';
        let history = [];
        let isScientificMode = false;

        // Map button keys to keyboard keys (extended for scientific functions)
        const keyMap = {
            'Enter': 'Enter',
            'Escape': 'c',
            'Delete': 'c',
            '/': '/',
            '*': '*',
            '-': '-',
            '+': '+',
            '.': '.',
            '^': '^', 
            'p': 'π', // Custom keyboard mapping for pi
            'e': 'e', // Custom keyboard mapping for e
            '!': '!', // Custom keyboard mapping for factorial
            '(': '(',
            ')': ')',
            'Backspace': 'backspace',
        };
        for (let i = 0; i <= 9; i++) {
            keyMap[i.toString()] = i.toString();
        }

        // --- Core Logic Functions ---

        function updateDisplay() {
            display.textContent = currentExpression === '' ? '0' : currentExpression;
        }
        
        // Helper for factorial calculation
        function factorial(n) {
            if (n < 0 || n !== Math.floor(n)) return NaN; // Only integer >= 0
            if (n === 0) return 1;
            let result = 1;
            for (let i = 2; i <= n; i++) {
                result *= i;
            }
            return result;
        }

        // Helper to prepare the expression for safe evaluation (Handling scientific syntax)
        function prepareExpression(expression) {
            let prepared = expression
                .replace(/×/g, '*')
                .replace(/÷/g, '/')
                .replace(/%/g, '/100*') // % = /100 * (value)
                .replace(/\^/g, '**') // Power operator
                // Trigonometric functions are calculated in degrees (as commonly expected in calculators)
                .replace(/sin\(([^)]+)\)/g, (match, p1) => `Math.sin((${p1}) * (Math.PI / 180))`) 
                .replace(/cos\(([^)]+)\)/g, (match, p1) => `Math.cos((${p1}) * (Math.PI / 180))`) 
                .replace(/tan\(([^)]+)\)/g, (match, p1) => `Math.tan((${p1}) * (Math.PI / 180))`) 
                .replace(/log\(([^)]+)\)/g, 'Math.log10($1)') // Log base 10
                .replace(/ln\(([^)]+)\)/g, 'Math.log($1)') // Natural log
                .replace(/sqrt\(([^)]+)\)/g, 'Math.sqrt($1)')
                .replace(/π/g, 'Math.PI')
                .replace(/e/g, 'Math.E');

            // Handle Factorial (x!)
            // Find all numbers followed by ! and replace with factorial(number)
            prepared = prepared.replace(/(\d+)!/g, (match, p1) => `factorial(${p1})`);
            
            // Final safety check to remove hanging operators at the end
            prepared = prepared.replace(/[\+\-\*\/%]+$/, '');
            
            return prepared;
        }


        function calculate() {
            // FIX: Auto-close open parentheses before evaluation
            let openCount = (currentExpression.match(/\(/g) || []).length;
            let closeCount = (currentExpression.match(/\)/g) || []).length;
            
            let expressionToEvaluate = currentExpression;

            if (openCount > closeCount) {
                const missingClosers = openCount - closeCount;
                for (let i = 0; i < missingClosers; i++) {
                    expressionToEvaluate += ')';
                }
                // Optional: Update display to show auto-closed expression
                // currentExpression = expressionToEvaluate; 
            }

            try {
                const expressionToEval = prepareExpression(expressionToEvaluate);
                
                // Use a Function constructor for safer evaluation
                const result = Function(`'use strict'; 
                    // Make factorial available within the scope
                    const factorial = ${factorial.toString()}; 
                    return (${expressionToEval});
                `)();
                
                if (result === undefined || isNaN(result) || !isFinite(result)) {
                    throw new Error("Invalid calculation");
                }
                
                // Format result to prevent floating point issues and keep it clean
                const formattedResult = parseFloat(result.toFixed(10)).toString();

                addToHistory(currentExpression, formattedResult);
                currentExpression = formattedResult;

            } catch (e) {
                console.error("Calculation error:", e);
                currentExpression = 'Error';
                // Automatically clear error after a short delay
                setTimeout(() => {
                    currentExpression = '0';
                    updateDisplay();
                }, 1500);
            }
        }

        function addToHistory(expression, result) {
            // Check if the calculation was already stored (e.g., repeated = clicks)
            if (history.length > 0 && history[0].expression === expression && history[0].result === result) {
                return; 
            }
            history.unshift({ expression: expression, result: result });
            // Keep history limited (e.g., last 10 entries)
            if (history.length > 10) {
                history.pop();
            }
            renderHistory();
        }

        function renderHistory() {
            historyList.innerHTML = '';
            if (history.length === 0) {
                historyList.innerHTML = '<li id="history-empty">No history yet.</li>';
                return;
            }

            history.forEach((item, index) => {
                const li = document.createElement('li');
                li.innerHTML = `<div class="history-expression">${item.expression} =</div><div class="history-result">${item.result}</div>`;
                li.dataset.index = index;
                li.addEventListener('click', () => {
                    // Clicking a history item loads the result into the current expression
                    currentExpression = item.result;
                    updateDisplay();
                });
                historyList.appendChild(li);
            });
        }

        function handleInput(key) {
            const operators = ['+', '-', '×', '÷', '%', '^'];
            const lastChar = currentExpression.slice(-1);
            const isFunction = ['sin', 'cos', 'tan', 'log', 'ln', 'sqrt'].includes(key);

            if (currentExpression === 'Error' && key !== 'c') {
                return; // Ignore input until cleared
            }

            switch (key) {
                case 'c':
                    currentExpression = '0';
                    break;
                case 'backspace':
                    currentExpression = currentExpression.length <= 1 || currentExpression.endsWith('Error') ? '0' : currentExpression.slice(0, -1);
                    break;
                case 'Enter': 
                case '=':
                    if (currentExpression !== 'Error' && !operators.includes(lastChar) && !['(', '.'].includes(lastChar)) { 
                        calculate();
                    }
                    break;
                case 'π':
                case 'e':
                    if (currentExpression === '0') {
                        currentExpression = key;
                    } else if (/\d$|\)$/.test(lastChar)) { 
                        currentExpression += '*' + key;
                    } else if (operators.includes(lastChar) || lastChar === '(') {
                        currentExpression += key;
                    } else {
                        currentExpression = key; 
                    }
                    break;
                case '(':
                    // Allow appending '(' after operator or at start
                    if (currentExpression === '0' || operators.includes(lastChar)) {
                        currentExpression += key;
                    } else if (/\d$/.test(lastChar) || lastChar === ')') {
                        currentExpression += '*' + key;
                    } else {
                        currentExpression += key;
                    }
                    break;
                case ')':
                    // Only append ')' if a matching '(' exists and the last char isn't an operator
                    if (currentExpression.includes('(') && !operators.includes(lastChar) && lastChar !== '(') {
                        currentExpression += key;
                    }
                    break;
                case '!':
                    // Only allow after a number or closing parenthesis
                    if (/\d$|\)$/.test(currentExpression)) {
                        currentExpression += key;
                    }
                    break;
                case '+':
                case '-':
                case '*':
                case '/':
                case '%':
                case '^':
                    // Map keys to display symbols
                    const operatorDisplay = key === '*' ? '×' : (key === '/' ? '÷' : key);

                    if (operators.includes(lastChar)) {
                        // Replace last operator
                        currentExpression = currentExpression.slice(0, -1) + operatorDisplay;
                    } else if (currentExpression === '0') {
                        if (key === '-') { // Allow negative sign at start
                           currentExpression = '-';
                        }
                    } else {
                        currentExpression += operatorDisplay;
                    }
                    break;
                case '.':
                    // Prevent multiple decimal points in the current number block
                    const parts = currentExpression.split(/[\+\-×÷%\^]/);
                    const lastNum = parts[parts.length - 1];
                    // FIX: Ensure '.' is not added after a function or constant
                    if (!lastNum.includes('.') && !lastNum.endsWith('π') && !lastNum.endsWith('e')) {
                        currentExpression += key;
                    }
                    break;
                default:
                    if (isFunction) {
                         // Scientific function always starts a new block, followed by '('
                         if (currentExpression === '0') {
                            currentExpression = `${key}(`;
                         } else if (operators.includes(lastChar) || lastChar === '(') {
                             currentExpression += `${key}(`;
                         } else if (/\d$|\)$/.test(lastChar)) {
                            // FIX: If preceded by number or ')' implicitly multiply: 5sin( -> 5*sin(
                            currentExpression += `*${key}(`;
                         } else {
                            // If appending after a number, overwrite or append as multiplication if not '0'
                            currentExpression = `${key}(`;
                         }
                    } else {
                        // Handle numbers
                        // If appending a number after constant or ')' implicitly multiply: 5(5) -> 5*(5)
                        if (/\) $|[πe]$/.test(lastChar)) {
                             currentExpression += '*' + key;
                        } else if (currentExpression === '0') {
                            currentExpression = key;
                        } else {
                            currentExpression += key;
                        }
                    }
                    break;
            }
            updateDisplay();
        }
        
        // --- Mode Toggle Logic ---
        function toggleMode() {
            isScientificMode = !isScientificMode;
            if (isScientificMode) {
                scientificBtns.style.display = 'grid';
                modeToggleBtn.textContent = 'Switch to Standard';
            } else {
                scientificBtns.style.display = 'none';
                modeToggleBtn.textContent = 'Switch to Scientific';
            }
        }


        // --- Event Listeners ---

        // FIX: Use Event Delegation on the parent wrapper for all calculator buttons.
        // This ensures button clicks work instantly, regardless of visibility state changes.
        buttonsWrapper.addEventListener('click', (event) => {
            const button = event.target.closest('.calc-button');
            if (button && button.id !== 'mode-toggle') {
                const key = button.getAttribute('data-key');
                handleInput(key);
            }
        });

        // Mode Toggle Click (Still directly attached as it's a dedicated button)
        modeToggleBtn.addEventListener('click', toggleMode);


        // Keyboard Input (This is already robust and works regardless of mode, as it targets document)
        document.addEventListener('keydown', (event) => {
            const key = event.key;
            // Check if key is a mapped key (including numbers, operators, Enter, etc.)
            if (key in keyMap) {
                event.preventDefault(); 
                handleInput(keyMap[key]);
            } else if (key === 'Delete') {
                // Map Delete key to clear 'c'
                event.preventDefault(); 
                handleInput('c');
            }
        });
        
        // Initialize
        updateDisplay();
        renderHistory();
    </script>
    "#;

    // Use the reusable function to wrap the content with the base HTML and Nav Bar
    render_base_page("Calculator", &format!("{}{}", style, html_content), current_theme)
}