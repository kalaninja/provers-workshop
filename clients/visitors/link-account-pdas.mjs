// @codama/renderers-rust only emits `Account::find_pda` / `create_pda` for an
// account that is linked to a PDA, and the ticketing IDL ships the program's
// PDAs unlinked. Every ticketing account is itself a PDA with a same-named
// `pdaNode`, so link them here to make the generated client derive its own
// account PDAs.
import { bottomUpTransformerVisitor, pdaLinkNode } from 'codama';

const PDA_ACCOUNTS = new Set(['event', 'ticket']);

export default () =>
  bottomUpTransformerVisitor([
    (node) =>
      node.kind === 'accountNode' && PDA_ACCOUNTS.has(node.name) && !node.pda
        ? { ...node, pda: pdaLinkNode(node.name) }
        : node,
  ]);
