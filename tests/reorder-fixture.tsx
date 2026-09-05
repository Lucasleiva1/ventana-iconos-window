import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { DrawerItemGrid } from '../src/components/drawer/DrawerItemGrid';
import { drawerApi } from '../src/services/drawerApi';
import '../src/styles/global.css';
const itemTypes = { a: 'file', b: 'folder', c: 'shortcut', d: 'file' } as const;
const items = ['a', 'b', 'c', 'd'].map((id, order) => ({ id, order, displayName: id, path: id, physicalName: id, available: true, storageMode: 'managed', type: itemTypes[id as keyof typeof itemTypes], iconKey: id }));
const drawer = { id: 'test', iconSize: 'medium', name: 'Test' };
drawerApi.getLevelItemIcon = async () => null;
drawerApi.openLevelItem = async (...args) => {
  const opened = ((window as any).openedItems ??= []);
  opened.push(args[2]);
};
drawerApi.reorderLevel = async (_drawer, _path, ids) => {
  (window as any).savedOrder = ids;
  return { rootDrawerId: 'test', relativePath: '', items: ids.map((id, order) => ({ ...items.find(item => item.id === id), order })) } as any;
};
function Fixture() {
  const [level, setLevel] = useState<any>({ rootDrawerId: 'test', relativePath: '', items });
  return <DrawerItemGrid drawer={drawer as any} drawers={[]} level={level} onNavigate={() => {}} onLevelChanged={next => next && setLevel(next)} onFeedback={message => { throw new Error(message); }} />;
}
createRoot(document.getElementById('root')!).render(<Fixture />);
