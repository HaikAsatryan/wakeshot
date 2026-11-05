import React from 'react';

/** Whether this render is a draft: stand-ins allowed and marked. The final render sets it false and refuses them. */
export const DraftContext = React.createContext(true);
