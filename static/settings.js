(function() {
    document.addEventListener('DOMContentLoaded', () => {
        const settings = document.getElementById('floating-settings');
        if (!settings) return;

        const handle = document.getElementById('settings-drag-handle');
        const form = settings.querySelector('.settings-form');
        const styleElement = document.getElementById('current-theme-vars');
        if (!handle || !form || !styleElement) return;

        const themeInputs = form.querySelectorAll('input[type="color"]');
        const fontSizeNumericInputs = form.querySelectorAll('input[type="number"]');
        const fontFamilyInput = document.getElementById('font_family');
        const modeInputs = form.querySelectorAll('input[name="theme_mode"]');
        const customColorControls = document.getElementById('custom-color-controls');
        const customThemeControls = form.querySelectorAll('.custom-theme-controls');
        const loadThemeSelect = document.getElementById('load_theme');
        const loadThemeNameInput = document.getElementById('theme_name_input');
        const themeNameInput = document.getElementById('theme_name');

        const palettes = {
            light: {
                primary_bg: '#f7f8fa',
                secondary_bg: '#ffffff',
                tertiary_bg: '#eef1f5',
                text_color: '#1f2933',
                link_visited: '#7c3aed',
                border_color: '#d8dee6',
            },
            dark: {
                primary_bg: '#1c1c1c',
                secondary_bg: '#111111',
                tertiary_bg: '#292929',
                text_color: '#ffffff',
                link_visited: '#b366ff',
                border_color: '#444444',
            },
        };

        function selectedMode() {
            const checked = form.querySelector('input[name="theme_mode"]:checked');
            return checked ? checked.value : 'custom';
        }

        function setInputValue(id, value) {
            const input = document.getElementById(id);
            if (input) input.value = value;
        }

        function clearLoadedTheme() {
            if (loadThemeSelect) loadThemeSelect.value = '';
            if (loadThemeNameInput) loadThemeNameInput.value = '';
        }

        function syncModeFields() {
            const mode = selectedMode();
            const accent = document.getElementById('accent_color')?.value || '#4da6ff';
            if (customColorControls) {
                customColorControls.classList.toggle('is-hidden', mode !== 'custom');
            }
            customThemeControls.forEach((el) => {
                el.classList.toggle('is-hidden', mode !== 'custom');
            });
            if (mode === 'light' || mode === 'dark') {
                const palette = palettes[mode];
                Object.keys(palette).forEach((key) => setInputValue(key, palette[key]));
                setInputValue('link_color', accent);
                setInputValue('link_hover', accent);
            }
        }

        window.toggleSettings = function() {
            if (settings.style.display === 'none') {
                settings.style.display = 'flex';
                localStorage.setItem('settings-visible', 'true');
            } else {
                settings.style.display = 'none';
                localStorage.setItem('settings-visible', 'false');
            }
        };

        const applyTheme = () => {
            syncModeFields();
            let cssVars = ':root {';
            themeInputs.forEach((input) => {
                cssVars += `--${input.id.replace(/_/g, '-')}: ${input.value};`;
            });

            const smallVal = document.getElementById('font_size_small').value;
            const mediumVal = document.getElementById('font_size_medium').value;
            const largeVal = document.getElementById('font_size_large').value;
            const marginVal = document.getElementById('element_margin').value;

            cssVars += `--font-size-small: ${smallVal}px;`;
            cssVars += `--font-size-medium: ${mediumVal}px;`;
            cssVars += `--font-size-large: ${largeVal}px;`;
            cssVars += `--element-margin: ${marginVal}px;`;
            cssVars += `--base-font-size: ${mediumVal}px;`;
            cssVars += `--base-font-family: ${fontFamilyInput.value};`;
            cssVars += '}';
            styleElement.innerHTML = cssVars;
        };

        themeInputs.forEach((input) => {
            input.addEventListener('input', () => {
                clearLoadedTheme();
                applyTheme();
            });
        });

        fontSizeNumericInputs.forEach((input) => {
            input.addEventListener('input', () => {
                clearLoadedTheme();
                applyTheme();
            });
        });

        if (fontFamilyInput) {
            fontFamilyInput.addEventListener('change', () => {
                clearLoadedTheme();
                applyTheme();
            });
        }

        modeInputs.forEach((input) => {
            input.addEventListener('change', () => {
                clearLoadedTheme();
                applyTheme();
            });
        });

        if (themeNameInput) {
            themeNameInput.addEventListener('input', clearLoadedTheme);
        }
        applyTheme();

        let isDragging = false;
        let currentX;
        let currentY;
        let initialX;
        let initialY;
        let xOffset = 0;
        let yOffset = 0;

        const savedPos = localStorage.getItem('settings-pos');
        if (savedPos) {
            const pos = JSON.parse(savedPos);
            xOffset = pos.x;
            yOffset = pos.y;
            settings.style.transform = `translate3d(${xOffset}px, ${yOffset}px, 0)`;
        }

        if (localStorage.getItem('settings-visible') === 'true') {
            settings.style.display = 'flex';
        }

        handle.addEventListener('mousedown', dragStart);
        document.addEventListener('mousemove', drag);
        document.addEventListener('mouseup', dragEnd);

        function dragStart(event) {
            initialX = event.clientX - xOffset;
            initialY = event.clientY - yOffset;
            if (event.target === handle || handle.contains(event.target)) {
                isDragging = true;
            }
        }

        function drag(event) {
            if (!isDragging) return;
            event.preventDefault();
            currentX = event.clientX - initialX;
            currentY = event.clientY - initialY;
            xOffset = currentX;
            yOffset = currentY;
            settings.style.transform = `translate3d(${currentX}px, ${currentY}px, 0)`;
        }

        function dragEnd() {
            initialX = currentX;
            initialY = currentY;
            isDragging = false;
            if (currentX !== undefined && currentY !== undefined) {
                localStorage.setItem('settings-pos', JSON.stringify({ x: currentX, y: currentY }));
            }
        }
    });
})();
