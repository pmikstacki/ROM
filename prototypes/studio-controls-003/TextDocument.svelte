<script>
 import {parse} from 'lossless-json';
 import {preflight} from './bounds.js';
 let text=$state('{"integer":18446744073709551615,"decimal":1.2300,"exponent":1e+03}'),accepted=$state(''),error=$state('');
 function apply(){try{preflight(text);parse(text);accepted=text;error='';}catch(err){error=err.message;}}
</script>
<label for="exact-document">JSON document</label><textarea id="exact-document" bind:value={text} rows="10" aria-invalid={!!error}></textarea><button onclick={apply}>Apply document</button><p role="alert">{error}</p><output data-testid="accepted">{accepted}</output>
<style>textarea{display:block;width:100%;font-family:monospace;margin:8px 0;padding:8px;border:1px solid #ddd;border-radius:5px}</style>
