import { render } from 'preact';
import { App } from './App';
import { applyTheme, readStoredTheme } from './lib/theme';
import './styles.css';

applyTheme(readStoredTheme());

const root = document.getElementById('app');
if (root) {
  render(<App />, root);
}
