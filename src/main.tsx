import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import './material-theme.css'
import './material-web'
import App from './App.tsx'

// Disable default right-click context menu globally
window.addEventListener('contextmenu', (e) => e.preventDefault());

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
