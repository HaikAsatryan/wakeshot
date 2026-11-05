import {loadFont} from '@remotion/fonts';
import mono400 from '@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2';
import {staticFile} from 'remotion';

// Instrument Sans is the brand's wordmark face (SemiBold, `docs/research/brand-identity.md` section 7); the rest of the
// video's words use the same family so the name and the lines read as one voice. The files sit in public/fonts/,
// copied from the designer's repo with their licence, so a render never fetches a font and every machine draws the
// same letters; a font that fails to load stops the render (`loadFont` cancels it). JetBrains Mono is kept only for
// the drawn terminal and the install command, where readers expect a code face.
export const SANS = '"Instrument Sans", sans-serif';
export const MONO = '"JetBrains Mono", monospace';

loadFont({family: 'Instrument Sans', url: staticFile('fonts/InstrumentSans-Regular.woff2'), weight: '400'});
loadFont({family: 'Instrument Sans', url: staticFile('fonts/InstrumentSans-SemiBold.woff2'), weight: '600'});
loadFont({family: 'JetBrains Mono', url: mono400, weight: '400'});
