<script>
 let Current=$state(null),mounted=$state(true),mode=$state('manual');
 let rows=$state([{id:'a',value:'same',draft:'same',error:''},{id:'b',value:'same',draft:'bad draft',error:'Invalid row b'},{id:'c',value:'third',draft:'third',error:''}]);
 async function load(next) {mode=next; Current=(await (next==='dnd'?import('./Sortable.svelte'):next==='text'?import('./TextDocument.svelte'):next==='json'?import('./Document.svelte'):import('./Manual.svelte'))).default;mounted=true;}
 load('manual');
</script>
<main><h1>Generic controls</h1><nav><button onclick={()=>load('manual')}>Manual</button><button onclick={()=>load('dnd')}>Sortable</button><button onclick={()=>load('json')}>Document</button><button onclick={()=>load('text')}>Exact textarea</button><button onclick={()=>mounted=!mounted}>Toggle editor</button></nav>
{#if mounted&&Current}{#if mode==='json'||mode==='text'}<Current />{:else}<Current bind:rows />{/if}{/if}
<output data-testid="values">{JSON.stringify(rows.map(row=>({id:row.id,value:row.value,draft:row.draft,error:row.error})))}</output></main>
<style>:global(body){font:14px system-ui;margin:24px;color:#222;background:#fff}main{max-width:800px}nav{display:flex;gap:8px;margin-bottom:16px}:global(button){background:#fff;border:1px solid #ddd;border-radius:5px;padding:7px;cursor:pointer}:global(input){padding:7px;border:1px solid #ddd;border-radius:5px}:global(li){display:flex;align-items:center;gap:8px;min-height:56px}:global(ol){padding:0;list-style:none}:global(.error){color:#b11}:global(output){display:block;overflow-wrap:anywhere;font-family:monospace;margin-top:16px}:global(.handle){touch-action:none;cursor:grab}</style>
