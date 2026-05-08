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
        const returnToInput = document.getElementById('theme_return_to');
        const importThemeBtn = document.getElementById('import-theme-btn');
        const importThemeFile = document.getElementById('import-theme-file');
        const exportThemeBtn = document.getElementById('export-theme-btn');

        if (returnToInput) {
            returnToInput.value = window.location.pathname + window.location.search;
        }

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

        function readThemeValue(id, fallback = '') {
            const input = document.getElementById(id);
            return input ? input.value : fallback;
        }

        function readThemeNumber(id, fallback = 0) {
            const value = Number.parseInt(readThemeValue(id), 10);
            return Number.isFinite(value) ? value : fallback;
        }

        function currentThemePayload() {
            syncModeFields();
            return {
                name: readThemeValue('theme_name', 'OGdevDesk Theme'),
                mode: selectedMode(),
                primary_bg: readThemeValue('primary_bg', '#1c1c1c'),
                secondary_bg: readThemeValue('secondary_bg', '#111111'),
                tertiary_bg: readThemeValue('tertiary_bg', '#292929'),
                text_color: readThemeValue('text_color', '#ffffff'),
                accent_color: readThemeValue('accent_color', '#4da6ff'),
                hover_window_accent: readThemeValue('hover_window_accent', readThemeValue('accent_color', '#4da6ff')),
                link_color: readThemeValue('link_color', readThemeValue('accent_color', '#4da6ff')),
                link_visited: readThemeValue('link_visited', '#b366ff'),
                link_hover: readThemeValue('link_hover', readThemeValue('accent_color', '#4da6ff')),
                border_color: readThemeValue('border_color', '#444444'),
                font_size_small: readThemeNumber('font_size_small', 14),
                font_size_medium: readThemeNumber('font_size_medium', 16),
                font_size_large: readThemeNumber('font_size_large', 18),
                element_margin: readThemeNumber('element_margin', 10),
                nav_height: readThemeNumber('nav_height', 50),
                font_family: fontFamilyInput?.value || 'sans-serif',
            };
        }

        function slugifyThemeName(name) {
            return String(name || 'ogdevdesk-theme')
                .trim()
                .toLowerCase()
                .replace(/[^a-z0-9]+/g, '-')
                .replace(/^-+|-+$/g, '') || 'ogdevdesk-theme';
        }

        function exportCurrentTheme() {
            const theme = currentThemePayload();
            const payload = {
                type: 'ogdevdesk.appearance.theme',
                version: 1,
                exported_at: new Date().toISOString(),
                theme,
            };
            const blob = new Blob([`${JSON.stringify(payload, null, 2)}\n`], { type: 'application/json' });
            const link = document.createElement('a');
            link.href = URL.createObjectURL(blob);
            link.download = `${slugifyThemeName(theme.name)}.ogdevdesk-theme.json`;
            document.body.appendChild(link);
            link.click();
            link.remove();
            URL.revokeObjectURL(link.href);
        }

        function applyImportedTheme(theme) {
            if (!theme || typeof theme !== 'object') {
                throw new Error('Theme file did not contain a valid theme object.');
            }

            const importedMode = ['light', 'dark', 'custom'].includes(theme.mode) ? theme.mode : 'custom';
            const modeInput = form.querySelector(`input[name="theme_mode"][value="${importedMode}"]`);
            if (modeInput) modeInput.checked = true;

            [
                'theme_name',
                'primary_bg',
                'secondary_bg',
                'tertiary_bg',
                'text_color',
                'accent_color',
                'hover_window_accent',
                'link_color',
                'link_visited',
                'link_hover',
                'border_color',
                'font_size_small',
                'font_size_medium',
                'font_size_large',
                'element_margin',
                'nav_height',
                'font_family',
            ].forEach((id) => {
                const input = document.getElementById(id);
                const themeKey = id === 'theme_name' ? 'name' : id;
                if (!input || theme[themeKey] === undefined || theme[themeKey] === null) return;
                input.value = theme[themeKey];
            });

            clearLoadedTheme();
            applyTheme();
        }

        async function loadThemeImport(file) {
            const raw = await file.text();
            const parsed = JSON.parse(raw);
            const theme = parsed.theme && typeof parsed.theme === 'object' ? parsed.theme : parsed;
            applyImportedTheme(theme);
        }

        function syncModeFields() {
            const mode = selectedMode();
            const accent = document.getElementById('accent_color')?.value || '#4da6ff';
            const hoverWindowAccent = document.getElementById('hover_window_accent')?.value || accent;
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
                setInputValue('hover_window_accent', hoverWindowAccent);
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
            const navHeightVal = document.getElementById('nav_height').value;

            cssVars += `--font-size-small: ${smallVal}px;`;
            cssVars += `--font-size-medium: ${mediumVal}px;`;
            cssVars += `--font-size-large: ${largeVal}px;`;
            cssVars += `--element-margin: ${marginVal}px;`;
            cssVars += `--nav-height: ${navHeightVal}px;`;
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

        if (exportThemeBtn) {
            exportThemeBtn.addEventListener('click', exportCurrentTheme);
        }

        if (importThemeBtn && importThemeFile) {
            importThemeBtn.addEventListener('click', () => importThemeFile.click());
            importThemeFile.addEventListener('change', async () => {
                const file = importThemeFile.files?.[0];
                if (!file) return;

                try {
                    await loadThemeImport(file);
                } catch (error) {
                    window.alert(`Theme import failed: ${error.message}`);
                } finally {
                    importThemeFile.value = '';
                }
            });
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
        settings.addEventListener('dblclick', centerOnEdgeDoubleClick);
        document.addEventListener('mousemove', drag);
        document.addEventListener('mouseup', dragEnd);

        function isOuterEdgeClick(event, element) {
            if (event.target.closest('button, input, textarea, select, a')) return false;
            const rect = element.getBoundingClientRect();
            const edge = 18;
            const onEdge = event.clientX - rect.left <= edge ||
                rect.right - event.clientX <= edge ||
                event.clientY - rect.top <= edge ||
                rect.bottom - event.clientY <= edge;
            return onEdge || handle.contains(event.target);
        }

        function centerOnEdgeDoubleClick(event) {
            if (!isOuterEdgeClick(event, settings)) return;

            const rect = settings.getBoundingClientRect();
            const targetLeft = Math.max(0, (window.innerWidth - rect.width) / 2);
            const targetTop = Math.max(0, (window.innerHeight - rect.height) / 2);
            xOffset += targetLeft - rect.left;
            yOffset += targetTop - rect.top;
            currentX = xOffset;
            currentY = yOffset;
            settings.style.transform = `translate3d(${xOffset}px, ${yOffset}px, 0)`;
            localStorage.setItem('settings-pos', JSON.stringify({ x: xOffset, y: yOffset }));
        }

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
