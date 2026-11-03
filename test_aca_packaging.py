"""Native packaging regression: no Cargo compilation, no project or save writes."""
from pathlib import Path
import tempfile
import unittest
import aca_build_web as package


class AcademyPackage(unittest.TestCase):
    def test_actual_source_shell_publishes_bindings_and_retains_bridge(self):
        source=(package.ROOT/'apps/soundcraft-web/index.html').read_text(encoding='utf-8')
        html=package.browser_html(source)
        self.assertIn('import init, * as bindings',html)
        self.assertIn('window.wasmBindings = bindings',html)
        self.assertIn('<script src="academy-bridge.js"></script>',html)
        self.assertNotIn('data-trunk',html)
        self.assertIn('ACA Audio Lab',html)

    def test_missing_native_export_or_bridge_fails_package(self):
        with tempfile.TemporaryDirectory(prefix='aca-packaging-') as temporary:
            dist=Path(temporary)
            (dist/'index.html').write_text(package.browser_html('<script src="academy-bridge.js"></script></body>'),encoding='utf-8')
            (dist/'academy-bridge.js').write_text('aca:ready aca:theme',encoding='utf-8')
            names=['academyTheme','academyReady','academyThemeState']
            (dist/'soundcraft_web.js').write_text('\n'.join('export function '+name+'() {}' for name in names),encoding='utf-8')
            package.verify_browser_package(dist)
            (dist/'soundcraft_web.js').write_text('export function academyTheme() {}',encoding='utf-8')
            with self.assertRaisesRegex(ValueError,'WASM export'):
                package.verify_browser_package(dist)


if __name__=='__main__':unittest.main()
