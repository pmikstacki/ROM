import test from 'node:test';import assert from 'node:assert/strict';
import { sqliteProfileEnvironment, SQLITE_PROFILE, requireSqliteHeader } from './sqlite-profile.mjs';
test('public3534 profile clears inherited native and Rust overrides rather than accepting private library paths',()=>{
 const env=sqliteProfileEnvironment({PATH:'/tool/bin',PKG_CONFIG_PATH:'/private',PKG_CONFIG_LIBDIR:'/private/lib',SQLITE3_LIB_DIR:'/old',SQLITE3_INCLUDE_DIR:'/old',LIBSQLITE3_SYS_USE_PKG_CONFIG:'0',RUSTFLAGS:'--cfg bad',CARGO_ENCODED_RUSTFLAGS:'bad'},'/owned/sqlite');
 assert.equal(env.PKG_CONFIG_PATH,'/owned/sqlite/lib/pkgconfig');assert.equal(env.SQLITE3_LIB_DIR,'/owned/sqlite/lib');assert.equal(env.SQLITE3_INCLUDE_DIR,'/owned/sqlite/include');assert.equal(env.ROM_EXPECT_SQLITE_VERSION,'3.53.4');assert.equal(env.LIBSQLITE3_SYS_USE_PKG_CONFIG,'1');assert.equal(env.PKG_CONFIG_LIBDIR,undefined);assert.equal(env.RUSTFLAGS,undefined);assert.equal(env.CARGO_ENCODED_RUSTFLAGS,undefined);
});
test('3534 header must identify exact primary-source version and source ID',()=>{
 const header=`#define SQLITE_VERSION "3.53.4"\n#define SQLITE_SOURCE_ID "${SQLITE_PROFILE.source_id}"\n`;
 assert.equal(requireSqliteHeader(header),true);assert.throws(()=>requireSqliteHeader(header.replace('3.53.4','3.53.2')),/profile source identity/);assert.throws(()=>requireSqliteHeader(header.replace(SQLITE_PROFILE.source_id,'unrelated')),/profile source identity/);
});

test('engine witness requires the selected executed test and nonempty successful test summary', async () => {
 const {requireSqliteEngineExecution}=await import('./sqlite-profile.mjs');
 const text=`Rust-linked SQLite engine: 3.53.4\nRust-linked SQLite source ID: ${SQLITE_PROFILE.source_id}\nRust-linked SQLite compile options: COMPILER=gcc-13.3.0,ENABLE_COLUMN_METADATA,THREADSAFE=1\ntest sqlite_native_engine_matches_selected_build_profile ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;\n`;
 assert.equal(requireSqliteEngineExecution(text),true);
 for(const changed of [text.split('\n')[0],text.replace('1 passed','0 passed'),text.replace('0 failed','1 failed'),text.replace('0 ignored','1 ignored'),text.replace(' ... ok',' ... ignored'),text.replace('3.53.4','3.53.2')]) assert.throws(()=>requireSqliteEngineExecution(changed),/engine was not executed/);
});


test('linked engine rejects same version with wrong source ID, missing compile options or duplicate identity lines', async () => {
 const {requireSqliteEngineExecution}=await import('./sqlite-profile.mjs');
 const version='Rust-linked SQLite engine: 3.53.4';
 const source='Rust-linked SQLite source ID: '+SQLITE_PROFILE.source_id;
 const options='Rust-linked SQLite compile options: COMPILER=gcc-13.3.0,ENABLE_COLUMN_METADATA,THREADSAFE=1';
 const result='test sqlite_native_engine_matches_selected_build_profile ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n';
 const text=[version,source,options,result].join('\n');
 assert.equal(requireSqliteEngineExecution(text),true);
 for(const changed of [
   text.replace(SQLITE_PROFILE.source_id,'2026-07-24 19:02:57 '+'a'.repeat(64)),
   text.replace(source+'\n',''),text.replace(options+'\n',''),
   text.replace('ENABLE_COLUMN_METADATA,',''),text.replace('THREADSAFE=1','THREADSAFE=0'),
   text.replace(options,options+',THREADSAFE=0'),text.replace(options,options+',THREADSAFE=1'),
   text+'\n'+source,text+'\n'+options,text+'\n'+version,
   text.replace('COMPILER=gcc-13.3.0','COMPILER=gcc-13.3.0;unexpected'),
   text.replace(' ... ok',' ... ignored'),text.replace('0 ignored','1 ignored')
 ])assert.throws(()=>requireSqliteEngineExecution(changed),/engine was not executed/);
});
test('public engine environment overrides a foreign expected source ID with the official profile',()=>{
 const env=sqliteProfileEnvironment({ROM_EXPECT_SQLITE_SOURCE_ID:'foreign'},'/owned/sqlite');
 assert.equal(env.ROM_EXPECT_SQLITE_SOURCE_ID,SQLITE_PROFILE.source_id);
});
