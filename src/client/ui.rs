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
    component LineInput {
        width: 260px;
        height: 30px;
        in property <length> padding-width: 8px;
        out property <string> out-text;
        TextInput {
            width: (parent.width) - root.padding-width;
            height: (parent.height) - root.padding-width;
            single-line: true;
            font-size: 20px;
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

        in property <string> ping-state: "";
        out property <string> ip_address;
        callback pingClick();

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
                text:"Enter server IP:";
            }
            LineInput {
                width: 350px;
                padding-width: 8px;
                y: -4px;
                changed out-text => {
                    root.ip_address = self.out-text;
                }
            }
            Button {
                height: 40px;
                y:-8px;
                text: "Ping server";
                clicked => pingClick();
            }
        }

        Text {
            y: 140px;
            text: parent.ping-state;
        }
    }
}