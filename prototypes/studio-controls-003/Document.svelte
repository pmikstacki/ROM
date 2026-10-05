<script>
 import {JSONEditor,Mode} from 'svelte-jsoneditor';
 import {parse,stringify} from 'lossless-json';
 import {preflight} from './bounds.js';
 let content=$state({text:'{"integer":18446744073709551615,"decimal":1.2300,"exponent":1e+03,"text":"<img src=x onerror=alert(1)>"}'}),error=$state(''),accepted=$state('');
 let mode=$state(Mode.text);
 const parser={parse(text){preflight(text);return parse(text);},stringify};
 function change(next){content=next;}
 function apply(){try{let text='text' in content?content.text:stringify(content.json);preflight(text);parse(text);accepted=text;error='';}catch(err){error=err.message;}}
</script>
<section aria-label="Document editor"><button onclick={()=>mode=Mode.tree}>Tree mode</button><button onclick={()=>mode=Mode.text}>Text mode</button><JSONEditor {content} {parser} {mode} onChange={change} mainMenuBar={false} navigationBar={false} statusBar={false} maxDocumentSizeTextMode={65536} /><button onclick={apply}>Apply document</button><p role="alert">{error}</p><output data-testid="accepted">{accepted}</output></section>
