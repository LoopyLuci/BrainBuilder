import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { installDevTauriShim } from './devMock/installDevTauriShim';
import './styles/theme.css';
import './ui/ui.css';
import './styles.css';

installDevTauriShim();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
