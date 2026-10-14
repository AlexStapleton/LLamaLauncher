// One-off source formatter using Vite's existing PostCSS dependency.
import fs from 'node:fs';
import postcss from 'postcss';
const file=new URL('../src/frontend/styles.css',import.meta.url);
const tree=postcss.parse(fs.readFileSync(file,'utf8'));
function format(node,depth=0){const indent='  '.repeat(depth);if(node.type==='root')return node.nodes.map(child=>format(child,depth)).join('\n\n');if(node.type==='decl')return `${indent}${node.prop}: ${node.value}${node.important?' !important':''};`;if(node.type==='comment')return `${indent}/* ${node.text} */`;const title=node.type==='atrule'?`@${node.name}${node.params?' '+node.params:''}`:node.selector;if(!node.nodes)return `${indent}${title};`;return `${indent}${title} {\n${node.nodes.map(child=>format(child,depth+1)).join('\n')}\n${indent}}`;}
fs.writeFileSync(file,format(tree)+'\n');
