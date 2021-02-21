(function() {var implementors = {};
implementors["panorama"] = [{"text":"impl Sync for Config","synthetic":true,"types":[]},{"text":"impl Sync for MailAccountConfig","synthetic":true,"types":[]},{"text":"impl Sync for ImapConfig","synthetic":true,"types":[]},{"text":"impl Sync for TlsMethod","synthetic":true,"types":[]},{"text":"impl&lt;S, S2&gt; Sync for LoopExit&lt;S, S2&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;S: Sync,<br>&nbsp;&nbsp;&nbsp;&nbsp;S2: Sync,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;S&gt; !Sync for CommandManager&lt;S&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; !Sync for CommandManager&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for MailCommand","synthetic":true,"types":[]},{"text":"impl Sync for Table","synthetic":true,"types":[]},{"text":"impl Sync for Rect","synthetic":true,"types":[]}];
implementors["panorama_imap"] = [{"text":"impl Sync for NoParams","synthetic":true,"types":[]},{"text":"impl Sync for Params","synthetic":true,"types":[]},{"text":"impl Sync for Empty","synthetic":true,"types":[]},{"text":"impl Sync for Messages","synthetic":true,"types":[]},{"text":"impl Sync for Attributes","synthetic":true,"types":[]},{"text":"impl Sync for Modifiers","synthetic":true,"types":[]},{"text":"impl Sync for CommandBuilder","synthetic":true,"types":[]},{"text":"impl Sync for Command","synthetic":true,"types":[]},{"text":"impl&lt;T&gt; Sync for SelectCommand&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Sync,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;T&gt; Sync for FetchCommand&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Sync,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;C&gt; Sync for Client&lt;C&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;C: Send + Sync,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl Sync for GreetingWaiter","synthetic":true,"types":[]},{"text":"impl&lt;'a, C&gt; Sync for ExecWaiter&lt;'a, C&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;C: Send + Sync,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl Sync for ClientConfig","synthetic":true,"types":[]},{"text":"impl Sync for ClientConfigBuilder","synthetic":true,"types":[]},{"text":"impl Sync for ClientUnauthenticated","synthetic":true,"types":[]},{"text":"impl Sync for ClientUnauthenticatedUnencrypted","synthetic":true,"types":[]},{"text":"impl Sync for ClientUnauthenticatedEncrypted","synthetic":true,"types":[]},{"text":"impl Sync for Command","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyStructParser&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for EntryParseStage&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for Response","synthetic":true,"types":[]},{"text":"impl Sync for Capability","synthetic":true,"types":[]},{"text":"impl Sync for RequestId","synthetic":true,"types":[]},{"text":"impl Sync for Status","synthetic":true,"types":[]},{"text":"impl Sync for ResponseCode","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for Request&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for AttrMacro","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for Response&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for Status","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for ResponseCode&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for UidSetMember","synthetic":true,"types":[]},{"text":"impl Sync for StatusAttribute","synthetic":true,"types":[]},{"text":"impl Sync for Metadata","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for MailboxDatum&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for Capability&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for Attribute","synthetic":true,"types":[]},{"text":"impl Sync for MessageSection","synthetic":true,"types":[]},{"text":"impl Sync for SectionPath","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for AttributeValue&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyStructure&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyContentCommon&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyContentSinglePart&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for ContentType&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for ContentDisposition&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for ContentEncoding&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyExtension&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for Envelope&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for Address&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Sync for RequestId","synthetic":true,"types":[]},{"text":"impl Sync for State","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyFields&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyExt1Part&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Sync for BodyExtMPart&lt;'a&gt;","synthetic":true,"types":[]}];
implementors["panorama_strings"] = [{"text":"impl&lt;T&gt; Sync for Store&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Sync,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;T&gt; Sync for Entry&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Sync,&nbsp;</span>","synthetic":true,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()