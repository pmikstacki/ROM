import {openSync,closeSync,readFileSync,fstatSync,lstatSync,constants} from 'node:fs';
export function mountAcknowledged(identity){
 const fd=openSync(identity.path,constants.O_RDONLY|constants.O_NOFOLLOW);
 try{const before=fstatSync(fd),current=lstatSync(identity.path);if(!before.isFile()||before.nlink!==1||before.dev!==identity.device||before.ino!==identity.inode||before.uid!==identity.uid||(before.mode&0o777)!==0o600||before.size>6||current.ino!==before.ino||current.dev!==before.dev)throw Error('exclusive mount acknowledgement changed');const value=readFileSync(fd,'utf8'),after=fstatSync(fd);if(after.ino!==before.ino||after.dev!==before.dev||after.size>6||!['','ready\n'].includes(value))throw Error('invalid mount acknowledgement');return value==='ready\n';}finally{closeSync(fd);}
}
