import {leaf as alias} from './leaf';
// 猫 😀
export function run(flag:boolean,parameter:number){
  alias(); alias(1); alias(parameter); alias(alias(1));
  return alias(flag?alias(1):alias(2));
}
export function promised(){return Promise.resolve(1);}
export async function scheduled(){return 1;}
export function* generator(){yield 1;}
export async function* mixed(){yield 1;}
export function outer(){async function nested(){await Promise.resolve(1);}return 1;}
export class Box {method(){return 1;}get value(){return 1;}}
export const callback=()=>1;
