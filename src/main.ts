import { mount } from 'svelte';
import App from './App.svelte';
import './style.css';
import './fonts.css';
import '@fontsource/space-mono/latin-400.css';
import './arcade.css';
import './pill.css';

mount(App, { target: document.getElementById('app')! });
