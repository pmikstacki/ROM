import{readdirSync,lstatSync,realpathSync,readFileSync,openSync,closeSync,fstatSync,writeFileSync,readSync,writeSync,fsyncSync,constants}from'node:fs';
import{createHash}from'node:crypto';
import{dirname}from'node:path';
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');

export function captureSourceTree(directory){
 if(typeof directory!=='string'||realpathSync(directory)!==directory||!lstatSync(directory).isDirectory())throw Error('canonical source directory required');
 const records=[];let total=0;
 function visit(path){for(const name of readdirSync(path).sort()){const child=path+'/'+name,value=lstatSync(child);if(value.isSymbolicLink())throw Error('regular source required');if(value.isDirectory()){visit(child);continue;}if(!value.isFile())throw Error('regular source required');if(value.size>1_048_576||records.length>=2048||(total+=value.size)>67_108_864)throw Error('source bound');const bytes=readFileSync(child);if(bytes.length!==value.size)throw Error('source changed during capture');records.push({path:child,sha256:hash(bytes)});}}
 visit(directory);return records;
}
export function requireSourceFence(before,after){if(JSON.stringify(before)!==JSON.stringify(after))throw Error('source fence changed');}

const binaryLimit=268_435_456;
function hashBinary(path){const fd=openSync(path,constants.O_RDONLY|constants.O_NOFOLLOW),digest=createHash('sha256'),buffer=Buffer.alloc(65_536);let total=0;try{const before=fstatSync(fd);if(!before.isFile()||before.size<=0||before.size>binaryLimit)throw Error('binary bound');for(;;){const count=readSync(fd,buffer,0,buffer.length,null);if(!count)break;if((total+=count)>binaryLimit)throw Error('binary bound');digest.update(buffer.subarray(0,count));}const after=fstatSync(fd);if(before.dev!==after.dev||before.ino!==after.ino||before.size!==after.size||before.mtimeMs!==after.mtimeMs||before.ctimeMs!==after.ctimeMs||total!==before.size)throw Error('binary changed during hashing');return{sha256:digest.digest('hex'),bytes:total,inode:after.ino,device:after.dev};}finally{closeSync(fd);}}
export function preserveBinary(source,destination){
 if(realpathSync(source)!==source||realpathSync(dirname(destination))!==dirname(destination))throw Error('canonical binary path required');
 const directory=lstatSync(dirname(destination));if(!directory.isDirectory()||(directory.mode&0o777)!==0o700||directory.uid!==process.getuid())throw Error('private binary directory required');
 const fd=openSync(source,constants.O_RDONLY|constants.O_NOFOLLOW);let output;
 try{const before=fstatSync(fd);if(!before.isFile()||before.size<=0||before.size>binaryLimit)throw Error('binary bound');output=openSync(destination,constants.O_WRONLY|constants.O_CREAT|constants.O_EXCL|constants.O_NOFOLLOW,0o700);const digest=createHash('sha256'),buffer=Buffer.alloc(65_536);let total=0;for(;;){const count=readSync(fd,buffer,0,buffer.length,null);if(!count)break;if((total+=count)>binaryLimit)throw Error('binary bound');digest.update(buffer.subarray(0,count));let written=0;while(written<count){const amount=writeSync(output,buffer,written,count-written);if(amount<=0)throw Error('binary copy stalled');written+=amount;}}fsyncSync(output);const after=fstatSync(fd);if(total!==before.size||before.dev!==after.dev||before.ino!==after.ino||before.size!==after.size||before.mtimeMs!==after.mtimeMs||before.ctimeMs!==after.ctimeMs)throw Error('binary changed during preservation');const saved=fstatSync(output),sha256=digest.digest('hex'),verified=hashBinary(destination);if(saved.nlink!==1||saved.uid!==process.getuid()||(saved.mode&0o777)!==0o700||verified.sha256!==sha256||verified.bytes!==total)throw Error('retained binary changed');return{path:destination,sha256,bytes:total,inode:saved.ino,device:saved.dev,source:{path:source,inode:before.ino,device:before.dev,links:before.nlink}};}finally{if(output!==undefined)closeSync(output);closeSync(fd);}
}

export function verifyPreservedBinary(identity){
 if(!identity||realpathSync(identity.path)!==identity.path)throw Error('canonical retained binary required');const value=lstatSync(identity.path),parent=lstatSync(dirname(identity.path));if(!value.isFile()||value.nlink!==1||value.uid!==process.getuid()||(value.mode&0o777)!==0o700||!parent.isDirectory()||parent.uid!==process.getuid()||(parent.mode&0o777)!==0o700)throw Error('private detached retained binary required');const current=hashBinary(identity.path);if(current.sha256!==identity.sha256||current.bytes!==identity.bytes||current.inode!==identity.inode||current.device!==identity.device)throw Error('retained binary identity changed');return{...identity,reused_immutable:true};
}
