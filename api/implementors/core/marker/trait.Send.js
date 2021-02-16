(function() {var implementors = {};
implementors["panorama"] = [{"text":"impl Send for Config","synthetic":true,"types":[]},{"text":"impl Send for MailAccountConfig","synthetic":true,"types":[]},{"text":"impl Send for ImapConfig","synthetic":true,"types":[]},{"text":"impl Send for TlsMethod","synthetic":true,"types":[]},{"text":"impl&lt;S, S2&gt; Send for LoopExit&lt;S, S2&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;S: Send,<br>&nbsp;&nbsp;&nbsp;&nbsp;S2: Send,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;S&gt; Send for CommandManager&lt;S&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;S: Send,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl Send for MailCommand","synthetic":true,"types":[]},{"text":"impl Send for Table","synthetic":true,"types":[]},{"text":"impl Send for Rect","synthetic":true,"types":[]}];
implementors["panorama_imap"] = [{"text":"impl Send for NoParams","synthetic":true,"types":[]},{"text":"impl Send for Params","synthetic":true,"types":[]},{"text":"impl Send for Empty","synthetic":true,"types":[]},{"text":"impl Send for Messages","synthetic":true,"types":[]},{"text":"impl Send for Attributes","synthetic":true,"types":[]},{"text":"impl Send for Modifiers","synthetic":true,"types":[]},{"text":"impl Send for CommandBuilder","synthetic":true,"types":[]},{"text":"impl Send for Command","synthetic":true,"types":[]},{"text":"impl&lt;T&gt; Send for SelectCommand&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Send,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;T&gt; Send for FetchCommand&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Send,&nbsp;</span>","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyStructParser&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for EntryParseStage&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for Request&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Send for AttrMacro","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for Response&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Send for Status","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for ResponseCode&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Send for UidSetMember","synthetic":true,"types":[]},{"text":"impl Send for StatusAttribute","synthetic":true,"types":[]},{"text":"impl Send for Metadata","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for MailboxDatum&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for Capability&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Send for Attribute","synthetic":true,"types":[]},{"text":"impl Send for MessageSection","synthetic":true,"types":[]},{"text":"impl Send for SectionPath","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for AttributeValue&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyStructure&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyContentCommon&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyContentSinglePart&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for ContentType&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for ContentDisposition&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for ContentEncoding&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyExtension&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for Envelope&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for Address&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl Send for RequestId","synthetic":true,"types":[]},{"text":"impl Send for State","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyFields&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyExt1Part&lt;'a&gt;","synthetic":true,"types":[]},{"text":"impl&lt;'a&gt; Send for BodyExtMPart&lt;'a&gt;","synthetic":true,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()