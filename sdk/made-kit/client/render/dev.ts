import { mount } from 'svelte';
import './app.css';
import Dev from './dev.svelte';

mount(Dev, { target: document.getElementById('app')! });
