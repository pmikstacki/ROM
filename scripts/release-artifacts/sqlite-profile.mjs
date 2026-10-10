// Selected public engine profile. SHA3 digests below are not SHA256 digests.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync, copyFileSync, lstatSync, realpathSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { extractCrate } from '../../tests/ai-flows-installed/crate-extraction.mjs';
import { recordedCommand } from './command-result.mjs';
import { hash } from '../skills/files.mjs';
import { runChild } from '../skills/process.mjs';
export const SQLITE_PROFILE=Object.freeze({version:'3.53.4',source_id:'2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc',source_url:'https://www.sqlite.org/2026/sqlite-autoconf-3530400.tar.gz',archive_sha3_256:'454e45f61c6bd75b7420e7190732dea03ce6639c63ada47bbc592f67fc340338',sqlite3_c_sha3_256:'67f423e9ebbbdc473cbc4772c872ee6b89f31fde4ed0279a5c25d5f65c043a16'});
export function sqliteProfileEnvironment(base,directory) {
  const env={...base};for(const key of Object.keys(env))if(key.startsWith('SQLITE3_')||key.startsWith('PKG_CONFIG_')||key.startsWith('LIBSQLITE3_')||['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS'].includes(key))delete env[key];
  return{...env,LIBSQLITE3_SYS_USE_PKG_CONFIG:'1',SQLITE3_STATIC:'1',PKG_CONFIG_PATH:join(directory,'lib/pkgconfig'),SQLITE3_LIB_DIR:join(directory,'lib'),SQLITE3_INCLUDE_DIR:join(directory,'include'),ROM_EXPECT_SQLITE_VERSION:SQLITE_PROFILE.version,ROM_EXPECT_SQLITE_SOURCE_ID:SQLITE_PROFILE.source_id};
}
export function requireSqliteHeader(header) {
  if(!header.includes(`#define SQLITE_VERSION        "${SQLITE_PROFILE.version}"`)&&!header.includes(`#define SQLITE_VERSION "${SQLITE_PROFILE.version}"`))throw Error('SQLite profile source identity mismatch');
  if(!header.includes(`"${SQLITE_PROFILE.source_id}"`))throw Error('SQLite profile source identity mismatch');return true;
}
export function requireSqliteEngineExecution(text) {
  const lines = text.split('\n');
  const values = prefix => lines.filter(line => line.startsWith(prefix)).map(line => line.slice(prefix.length));
  const versions = values('Rust-linked SQLite engine: ');
  const sources = values('Rust-linked SQLite source ID: ');
  const optionLines = values('Rust-linked SQLite compile options: ');
  const options = optionLines.length === 1 ? optionLines[0].split(',') : [];
  const names = options.map(option => option.split('=')[0]);
  const summaries = [...text.matchAll(/^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;/gm)];
  if (versions.length !== 1 || versions[0] !== SQLITE_PROFILE.version ||
      sources.length !== 1 || sources[0] !== SQLITE_PROFILE.source_id ||
      optionLines.length !== 1 || optionLines[0].length > 65536 || options.length > 512 ||
      options.some(option => !/^[A-Z][A-Z0-9_]*(?:=[A-Za-z0-9_.+-]+)?$/.test(option)) ||
      new Set(names).size !== options.length ||
      !['THREADSAFE=1', 'ENABLE_COLUMN_METADATA'].every(option => options.includes(option)) ||
      !lines.includes('test sqlite_native_engine_matches_selected_build_profile ... ok') ||
      summaries.length !== 1 || summaries[0][1] !== 'ok' || Number(summaries[0][2]) !== 1 || Number(summaries[0][3]) || Number(summaries[0][4]))
    throw Error('selected SQLite engine was not executed');
  return true;
}
const sha3=path=>createHash('sha3-256').update(readFileSync(path)).digest('hex');
function regular(path,max){const stat=lstatSync(path);if(resolve(path)!==path||realpathSync(path)!==path||!stat.isFile()||stat.nlink!==1||stat.size<1||stat.size>max)throw Error('invalid selected SQLite input');return stat;}
export async function prepareSqliteProfile(input,directory,runner=runChild) {
  if(!input||Object.keys(input).sort().join(',')!=='ar,archive,cc')throw Error('official SQLite profile inputs required');
  regular(input.archive,64*1024*1024);if(sha3(input.archive)!==SQLITE_PROFILE.archive_sha3_256)throw Error('official SQLite archive SHA3 mismatch');
  for(const key of ['cc','ar']){const stat=lstatSync(input[key]);if(realpathSync(input[key])!==input[key]||!stat.isFile()||!(stat.mode&0o111))throw Error('canonical SQLite compiler required');}
  mkdirSync(directory);mkdirSync(join(directory,'evidence'));mkdirSync(join(directory,'source'));mkdirSync(join(directory,'include'));mkdirSync(join(directory,'lib'));mkdirSync(join(directory,'lib/pkgconfig'));
  const archive=join(directory,'sqlite-autoconf-3530400.tar.gz');copyFileSync(input.archive,archive);
  const root=extractCrate(archive,join(directory,'source'),'sqlite-autoconf-3530400'),source=join(root,'sqlite3.c'),header=join(root,'sqlite3.h');
  if(sha3(source)!==SQLITE_PROFILE.sqlite3_c_sha3_256)throw Error('official SQLite C SHA3 mismatch');requireSqliteHeader(readFileSync(header,'utf8'));
  copyFileSync(header,join(directory,'include/sqlite3.h'));
  const env={PATH:'/run/current-system/sw/bin:/usr/bin:/bin',LANG:'C',LC_ALL:'C',TZ:'UTC'},tools=Object.fromEntries(['cc','ar'].map(key=>[key,{path:input[key],sha256:hash(input[key])}]));
  const bounded=(program,args,options)=>runner(program,args,{...options,env,timeout:240000,maxBytes:32*1024*1024});const commands=[];
  for(const [program,args,stem] of [[input.cc,['-O2','-fPIC','-DSQLITE_THREADSAFE=1','-DSQLITE_ENABLE_COLUMN_METADATA','-c',source,'-o',join(directory,'sqlite3.o')],'sqlite-compile'],[input.ar,['rcs',join(directory,'lib/libsqlite3.a'),join(directory,'sqlite3.o')],'sqlite-archive']]){
    const command=await recordedCommand(program,args,directory,directory,stem,bounded);commands.push(command);
    if(command.exit_code!==0||command.bounded_abort||command.spawn_failed)throw Error('SQLite profile build failed');
  }
  if(sha3(archive)!==SQLITE_PROFILE.archive_sha3_256||sha3(source)!==SQLITE_PROFILE.sqlite3_c_sha3_256||Object.entries(tools).some(([,tool])=>hash(tool.path)!==tool.sha256))throw Error('SQLite profile build inputs changed');
  regular(join(directory,'lib/libsqlite3.a'),64*1024*1024);
  writeFileSync(join(directory,'lib/pkgconfig/sqlite3.pc'),`prefix=${directory}\nlibdir=\${prefix}/lib\nincludedir=\${prefix}/include\n\nName: SQLite\nDescription: Selected public SQLite engine\nVersion: 3.53.4\nLibs: -L\${libdir} -lsqlite3 -lm -ldl -lpthread\nCflags: -I\${includedir}\n`,{flag:'wx'});
  const record={schema_version:1,profile:SQLITE_PROFILE,tools,commands,archive_sha256:hash(archive),sqlite3_c_sha256:hash(source),sqlite3_h_sha256:hash(header),library_sha256:hash(join(directory,'lib/libsqlite3.a')),pkg_config_sha256:hash(join(directory,'lib/pkgconfig/sqlite3.pc')),log_sha256:Object.fromEntries(commands.flatMap(command=>[command.stdout,command.stderr]).map(path=>[path,hash(join(directory,path))])),notice:'SQLite is in the public domain. Official source and licensing: https://www.sqlite.org/copyright.html. Original source archive is retained.',linked_engine_verified:false};
  writeFileSync(join(directory,'profile.json'),JSON.stringify(record,null,2)+'\n',{flag:'wx'});return{directory,record,environment:sqliteProfileEnvironment({},directory)};
}

export function admitSqliteProfileInput(path, sha256) {
  if(typeof path!=='string'||!/^[a-f0-9]{64}$/.test(sha256??''))throw Error('official SQLite profile inputs required');
  regular(path,65536);if(hash(path)!==sha256)throw Error('SQLite profile selection changed');
  const input=JSON.parse(readFileSync(path,'utf8'));
  if(Object.keys(input).sort().join(',')!=='ar,archive,cc')throw Error('official SQLite profile inputs required');
  regular(input.archive,64*1024*1024);if(sha3(input.archive)!==SQLITE_PROFILE.archive_sha3_256)throw Error('official SQLite archive SHA3 mismatch');
  return input;
}
export function fenceSqliteProfile(selected) {
  const {directory,record}=selected;
  if(sha3(join(directory,'sqlite-autoconf-3530400.tar.gz'))!==SQLITE_PROFILE.archive_sha3_256||sha3(join(directory,'source/sqlite-autoconf-3530400/sqlite3.c'))!==SQLITE_PROFILE.sqlite3_c_sha3_256||hash(join(directory,'include/sqlite3.h'))!==record.sqlite3_h_sha256||hash(join(directory,'lib/libsqlite3.a'))!==record.library_sha256||hash(join(directory,'lib/pkgconfig/sqlite3.pc'))!==record.pkg_config_sha256||Object.entries(record.log_sha256).some(([path,digest])=>hash(join(directory,path))!==digest))throw Error('selected SQLite profile changed');
  for(const tool of Object.values(record.tools))if(realpathSync(tool.path)!==tool.path||hash(tool.path)!==tool.sha256)throw Error('selected SQLite tool changed');
  requireSqliteHeader(readFileSync(join(directory,'include/sqlite3.h'),'utf8'));
}
