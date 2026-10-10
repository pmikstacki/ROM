import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { hash } from '../skills/files.mjs';
import { requireSqliteBuildCommands } from './sqlite-evidence.mjs';

function fixture() {
 const directory=mkdtempSync(join(tmpdir(),'rom-sqlite-evidence-'));mkdirSync(join(directory,'evidence'));
 const root='/owned/selected-engine',tools={cc:{path:'/tool/gcc',sha256:'a'.repeat(64)},ar:{path:'/tool/ar',sha256:'b'.repeat(64)}};
 const commands=[['cc',['-O2','-fPIC','-DSQLITE_THREADSAFE=1','-DSQLITE_ENABLE_COLUMN_METADATA','-c',join(root,'source/sqlite-autoconf-3530400/sqlite3.c'),'-o',join(root,'sqlite3.o')],'sqlite-compile'],['ar',['rcs',join(root,'lib/libsqlite3.a'),join(root,'sqlite3.o')],'sqlite-archive']].map(([name,args,stem],index)=>({program:tools[name].path,args,cwd:root,exit_code:0,bounded_abort:false,spawn_failed:false,started_at:`2026-10-09T00:0${index}:00.000Z`,finished_at:`2026-10-09T00:0${index}:01.000Z`,stdout:`evidence/${stem}.stdout.log`,stderr:`evidence/${stem}.stderr.log`}));
 const log_sha256={};for(const command of commands)for(const path of [command.stdout,command.stderr]){writeFileSync(join(directory,path),'');log_sha256[path]=hash(join(directory,path));}
 return{directory,record:{tools,commands,log_sha256}};
}
test('SQLite compile/archive acceptance binds exact recipe, tool identity, deadlines and log bytes',()=>{
 const {directory,record}=fixture();assert.equal(requireSqliteBuildCommands(record,directory),true);
 for(const mutate of [r=>r.commands.pop(),r=>r.commands[0].args.push('-DUNREVIEWED'),r=>r.commands[1].program='/other/ar',r=>r.commands[0].exit_code=1,r=>r.commands[0].finished_at='2026-10-09T00:04:00.001Z',r=>r.commands[0].bounded_abort=true,r=>delete r.log_sha256[r.commands[0].stdout]]){
 const changed=structuredClone(record);mutate(changed);assert.throws(()=>requireSqliteBuildCommands(changed,directory),/SQLite build evidence/);
 }
 writeFileSync(join(directory,record.commands[0].stdout),'new bytes');assert.throws(()=>requireSqliteBuildCommands(record,directory),/SQLite build evidence/);
});
