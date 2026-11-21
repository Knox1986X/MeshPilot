import React from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';
import App from './App';

const container = document.getElementById('root');
if (!container) throw new Error('Корневой элемент #root не найден');

createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
