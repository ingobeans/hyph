slint::slint! {
    import { Button } from "std-widgets.slint";
    component InnerBorder {
        in property <brush> background-color;
        in property <brush> border-color;
        Rectangle {
            background: parent.border-color;
        }
        Rectangle {
            width: (parent.width) - 2px;
            height: (parent.height) - 2px;
            background: parent.background-color;
        }
    }
    component OuterBorder {
        in property <brush> background-color;
        in property <brush> border-color;
        in property <length> padding-width;
        Rectangle {
            x: -parent.padding-width;
            y: -parent.padding-width;
            width: (parent.width) + 2px + parent.padding-width * 2;
            height: 2px;
            background: parent.border-color;
        }
        Rectangle {
            x: -parent.padding-width;
            y: -parent.padding-width;
            width: 2px;
            height: (parent.height) + 2px + parent.padding-width * 2;
            background: parent.border-color;
        }
        Rectangle {
            x:  -parent.padding-width;
            y: (parent.height) + parent.padding-width;
            width: (parent.width) + 2px + parent.padding-width * 2;
            height: 2px;
            background: parent.border-color;
        }
        Rectangle {
            x: parent.width + parent.padding-width;
            y:  -parent.padding-width;
            width: 2px;
            height: (parent.height) + 2px + parent.padding-width * 2;
            background: parent.border-color;
        }
    }
    component Container {
        @children
    }
    component LineInput {
        width: 260px;
        height: 30px;
        in property <length> padding-width: 8px;
        out property <string> out-text;
        in property <bool> enabled;
        TextInput {
            width: (parent.width) - root.padding-width;
            height: (parent.height) - root.padding-width;
            single-line: true;
            font-size: 20px;
            enabled: root.enabled;
            changed text => {
                parent.out-text = self.text;
            };
            OuterBorder {
                padding-width: root.padding-width;
                width: parent.width;
                height: parent.height;
                background-color: rgb(13, 17, 23);
                border-color: rgb(61, 68, 77);
            }
        }
    }
    export component App inherits Window {
        background: rgb(13, 17, 23);
        no-frame: true;
        title: "Hyph";
        width: 990px;
        height: 450px;

        in property <string> username: "Logged in as: USER";
        in property <bool> ping-enabled: true;
        in property <bool> start-exam-enabled: true;

        in property <bool> ping-success: false;
        in property <bool> ping-fail: false;

        in property <bool> start-exam-success: false;
        in property <bool> start-exam-fail: false;

        out property <string> ip_address;
        callback pingClick();
        callback startExamClick();

        InnerBorder {
            width: parent.width;
            height: parent.height;
            background-color: rgb(13, 17, 23);
            border-color: rgb(61, 68, 77);
        }

        Text {
            text: "Exam Controller";
            font-size: 32px;
            y: 10px;
        }

        HorizontalLayout {
            y: 80px;
            width: 640px;
            alignment: start;
            spacing: 10px;
            Text {
                font-size: 20px;
                text:"Server IP:";
            }
            LineInput {
                width: 420px;
                padding-width: 8px;
                y: -4px;
                enabled: root.start-exam-enabled;
                changed out-text => {
                    root.ip_address = self.out-text;
                }
            }
            Button {
                enabled: ping-enabled;
                height: 40px;
                y:-8px;
                text: "Ping server";
                clicked => pingClick();
            }
            Container {
                width: 30px;
                Image {
                    height: 30px;
                    width: 30px;
                    y:-5px;
                    visible: root.ping-success;
                    source: @image-url("assets/check.png");
                }
                Image {
                    height: 30px;
                    width: 30px;
                    y:-5px;
                    visible: root.ping-fail;
                    source: @image-url("assets/fail.png");
                }
            }
        }
        Text {
            text: username;
            y: 125px;
        }
        HorizontalLayout {
            y: 160px;
            width: 240px;
            alignment:start;
            spacing: 10px;
            Button {
                enabled: root.ping-success && root.ping-enabled && root.start-exam-enabled;
                width: 240px;
                height: 40px;
                y:-8px;
                text: "Start Exam";
                primary: true;
                clicked => startExamClick();
            }
            Container {
                width: 30px;
                Image {
                    height: 30px;
                    width: 30px;
                    y:-5px;
                    visible: root.start-exam-success;
                    source: @image-url("assets/check.png");
                }
                Image {
                    height: 30px;
                    width: 30px;
                    y:-5px;
                    visible: root.start-exam-fail;
                    source: @image-url("assets/fail.png");
                }
            }
        }
    }
}