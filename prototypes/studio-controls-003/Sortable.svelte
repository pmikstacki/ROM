<script>
 import {dragHandleZone,dragHandle,SHADOW_ITEM_MARKER_PROPERTY_NAME} from 'svelte-dnd-action';
 import {tick} from 'svelte';
 let {rows=$bindable()}=$props();let draft=$state(rows);
 function consider(event){draft=event.detail.items;}
 function finalize(event){draft=event.detail.items;rows=draft.filter(row=>!row[SHADOW_ITEM_MARKER_PROPERTY_NAME]);}
 async function move(id,delta){let i=rows.findIndex(row=>row.id===id),j=i+delta;if(j<0||j>=rows.length)return;let copy=[...rows];[copy[i],copy[j]]=[copy[j],copy[i]];rows=copy;draft=copy;await tick();document.querySelector(`[data-row="${id}"] input`).focus();}
</script>
<ol aria-label="Entries" use:dragHandleZone={{items:draft,type:'probe',dropFromOthersDisabled:true,flipDurationMs:0,delayTouchStart:80}} onconsider={consider} onfinalize={finalize}>{#each draft as row,i (row.id)}<li data-row={row.id} aria-label={'Entry '+row.id}><span class="handle" use:dragHandle aria-label={'Drag '+row.id}>⠿</span><label for={'dnd-'+row.id}>Entry {i+1}</label><input id={'dnd-'+row.id} bind:value={row.draft} aria-invalid={!!row.error}/><button aria-label={'Move '+row.id+' up'} disabled={i===0} onclick={()=>move(row.id,-1)}>↑</button><button aria-label={'Move '+row.id+' down'} disabled={i===rows.length-1} onclick={()=>move(row.id,1)}>↓</button><span class="error">{row.error}</span></li>{/each}</ol>
