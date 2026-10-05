<script>
 import {tick} from 'svelte';
 let {rows=$bindable()}=$props();
 async function move(id,delta){let i=rows.findIndex(row=>row.id===id),j=i+delta;if(j<0||j>=rows.length)return;let copy=[...rows];[copy[i],copy[j]]=[copy[j],copy[i]];rows=copy;await tick();document.querySelector(`[data-row="${id}"] input`).focus();}
</script>
<ol aria-label="Entries">{#each rows as row,i (row.id)}<li data-row={row.id}><label for={'manual-'+row.id}>Entry {i+1}</label><input id={'manual-'+row.id} bind:value={row.draft} aria-invalid={!!row.error}/><button aria-label={'Move '+row.id+' up'} disabled={i===0} onclick={()=>move(row.id,-1)}>↑</button><button aria-label={'Move '+row.id+' down'} disabled={i===rows.length-1} onclick={()=>move(row.id,1)}>↓</button><span class="error">{row.error}</span></li>{/each}</ol>
