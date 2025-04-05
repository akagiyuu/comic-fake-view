import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';
import { ThemeProvider } from '@/components/theme-provider';
import { Toaster } from 'sonner';

ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
        <ThemeProvider defaultTheme="system" storageKey="ui-theme">
            <App />
            <Toaster richColors closeButton position="top-right" />
        </ThemeProvider>
    </React.StrictMode>,
);
