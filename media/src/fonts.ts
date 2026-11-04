import {loadFont} from '@remotion/fonts';
import inter400 from '@fontsource/inter/files/inter-latin-400-normal.woff2';
import inter600 from '@fontsource/inter/files/inter-latin-600-normal.woff2';
import mono400 from '@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2';

// Fonts come from npm, not from the system, so a rebuild on any machine draws the same letters. Inter stands in for
// the brand's Segoe UI Variable (`docs/brand.md` names Inter as the fallback); the reason is in media/README.md.
export const SANS = 'Inter, sans-serif';
export const MONO = '"JetBrains Mono", monospace';

loadFont({family: 'Inter', url: inter400, weight: '400'});
loadFont({family: 'Inter', url: inter600, weight: '600'});
loadFont({family: 'JetBrains Mono', url: mono400, weight: '400'});
