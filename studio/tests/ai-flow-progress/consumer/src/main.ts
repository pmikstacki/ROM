import 'rom-studio/styles';
import { mount } from 'svelte';
import App from './App.svelte';
const target = document.getElementById('app');
if (!target) throw Error('missing consumer mount');
mount(App, { target });
