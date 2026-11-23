import { render } from 'preact';
import { App } from './App';
import { applyTheme, readStoredTheme } from './lib/theme';
import { installImeGuard } from './lib/ime';
import './styles.css';

applyTheme(readStoredTheme());
installImeGuard();

const root = document.getElementById('app');
if (root) {
  render(<App />, root);
}
