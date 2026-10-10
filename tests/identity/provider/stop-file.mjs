import { constants, openSync, closeSync, fstatSync, lstatSync, realpathSync, readSync, writeSync, fsyncSync } from 'node:fs';
import { dirname } from 'node:path';
function admitRoot(root,path){
 if(typeof root!=='string'||typeof path!=='string'||!path.startsWith(`${root}/private/`)||realpathSync(root)!==root||realpathSync(dirname(path))!==dirname(path))throw Error('owned private stop path required');
 const directory=lstatSync(root),parent=lstatSync(dirname(path));
 if(!directory.isDirectory()||!parent.isDirectory()||parent.dev!==directory.dev||parent.uid!==process.getuid()||(parent.mode&0o077)!==0)throw Error('owned private stop directory required');
}
function identity(path,stat){return{path,device:stat.dev,inode:stat.ino,birth_ms:stat.birthtimeMs};}
function admitFile(root,expected,stat){
 const actual=lstatSync(expected.path),directory=lstatSync(root);
 if(!actual.isFile()||actual.isSymbolicLink()||!stat.isFile()||actual.dev!==stat.dev||actual.ino!==stat.ino||stat.dev!==directory.dev||stat.dev!==expected.device||stat.ino!==expected.inode||stat.birthtimeMs!==expected.birth_ms||stat.uid!==process.getuid()||(stat.mode&0o777)!==0o600||stat.nlink!==1||stat.size>5)throw Error('owned stop identity changed');
}
export function createOwnedStopFile(root,path){
 admitRoot(root,path);const fd=openSync(path,constants.O_CREAT|constants.O_EXCL|constants.O_RDWR|constants.O_NOFOLLOW,0o600);
 try{const result=identity(path,fstatSync(fd));admitFile(root,result,fstatSync(fd));return result;}finally{closeSync(fd);}
}
function useFile(root,expected,write){
 admitRoot(root,expected.path);const fd=openSync(expected.path,(write?constants.O_RDWR:constants.O_RDONLY)|constants.O_NOFOLLOW);
 try{
  admitFile(root,expected,fstatSync(fd));const bytes=Buffer.alloc(6),length=readSync(fd,bytes,0,bytes.length,0),value=bytes.subarray(0,length).toString('utf8');
  if(value!==''&&value!=='stop\n')throw Error('closed stop file content required');
  if(write&&value===''){writeSync(fd,Buffer.from('stop\n'),0,5,0);fsyncSync(fd);}
  admitFile(root,expected,fstatSync(fd));return write||value==='stop\n';
 }finally{closeSync(fd);}
}
export function readOwnedStopFile(root,identity){return useFile(root,identity,false);}
export function requestOwnedStop(root,identity){return useFile(root,identity,true);}
